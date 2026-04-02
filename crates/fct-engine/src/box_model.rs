// ============================================================================
// TOKEN BOX MODEL (Phase 4)
// ============================================================================

use crate::errors::{EngineError, EngineResult};
use crate::r_dag::ExecutionMode;
use crate::tokenizer::Tokenizer;
use fct_ast::{OrderedMap, PipelineNode, ValueNode};
use fct_std::{LensContext, LensRegistry, TrustLevel};
use std::collections::HashMap;

/// Represents a logical prompt section with allocation attributes
#[derive(Debug, Clone)]
pub struct Section {
    pub id: String,
    pub source_index: usize,            // original section order index
    pub priority: i32,                  // lower = dropped earlier
    pub base_size: usize,               // FACET Units after initial render
    pub min: usize,                     // minimum guaranteed size
    pub grow: f64,                      // weight for distributing excess space
    pub shrink: f64,                    // weight for compression/removal
    pub strategy: Option<PipelineNode>, // compression lens pipeline
    pub content: ValueNode,             // actual content
    pub current_size: usize,            // current allocated size
    pub is_critical: bool,              // shrink == 0
}

impl Section {
    pub fn new(id: String, content: ValueNode, base_size: usize) -> Self {
        Self {
            id,
            source_index: usize::MAX,
            priority: 500, // default priority
            base_size,
            min: 0,
            grow: 0.0,
            shrink: 0.0,
            strategy: None,
            content,
            current_size: base_size,
            is_critical: false, // will be calculated
        }
    }

    pub fn from_content(id: String, content: ValueNode, tokenizer: &Tokenizer) -> Self {
        let base_size = tokenizer.count_facet_units_in_value(&content);
        Self {
            id,
            source_index: usize::MAX,
            priority: 500,
            base_size,
            min: 0,
            grow: 0.0,
            shrink: 0.0,
            strategy: None,
            content,
            current_size: base_size,
            is_critical: false,
        }
    }

    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_limits(mut self, min: usize, grow: f64, shrink: f64) -> Self {
        self.min = min;
        self.grow = grow;
        self.shrink = shrink;
        self.is_critical = shrink == 0.0;
        self
    }

    pub fn with_strategy(mut self, strategy: PipelineNode) -> Self {
        self.strategy = Some(strategy);
        self
    }

    /// Apply compression strategy and return new size
    pub fn apply_compression(
        &self,
        lens_registry: &LensRegistry,
        tokenizer: &Tokenizer,
        mode: ExecutionMode,
    ) -> EngineResult<(ValueNode, usize)> {
        if let Some(strategy) = &self.strategy {
            // Apply compression pipeline directly to content
            let mut current_value = self.content.clone();
            let ctx = LensContext {
                variables: HashMap::new(),
            };

            for lens_call in &strategy.lenses {
                let lens = lens_registry
                    .get(&lens_call.name)
                    .ok_or_else(|| EngineError::UnknownLens {
                        name: lens_call.name.clone(),
                    })?;
                let signature = lens.signature();
                if !signature.deterministic {
                    return Err(EngineError::LensExecutionFailed {
                        message: format!(
                            "Compression lens '{}' must be deterministic for layout strategy",
                            lens_call.name
                        ),
                    });
                }
                if mode == ExecutionMode::Pure
                    && signature.trust_level != TrustLevel::Pure
                {
                    return Err(EngineError::LensExecutionFailed {
                        message: format!(
                            "Compression lens '{}' disallowed in pure mode (Level-0 required)",
                            lens_call.name
                        ),
                    });
                }

                let kwargs: HashMap<String, ValueNode> = lens_call
                    .kwargs
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();

                current_value = lens
                    .execute(current_value, lens_call.args.clone(), kwargs, &ctx)
                    .map_err(|e| EngineError::LensExecutionFailed {
                        message: format!("Compression lens '{}' failed: {}", lens_call.name, e),
                    })?;
            }

            // Calculate FACET Units after compression
            let size = tokenizer.count_facet_units_in_value(&current_value);
            Ok((current_value, size))
        } else {
            Ok((self.content.clone(), self.current_size))
        }
    }
}

/// Result of Token Box Model allocation
#[derive(Debug, Clone)]
pub struct AllocationResult {
    pub sections: Vec<AllocatedSection>,
    pub total_size: usize,
    pub budget: usize,
    pub overflow: usize,
}

/// A section with its final allocated size
#[derive(Debug, Clone)]
pub struct AllocatedSection {
    pub section: Section,
    pub final_size: usize,
    pub was_compressed: bool,
    pub was_truncated: bool,
    pub was_dropped: bool,
}

/// Token Box Model implementation
pub struct TokenBoxModel {
    budget: usize,
    tokenizer: Tokenizer,
}

impl TokenBoxModel {
    pub fn new(budget: usize) -> Self {
        let tokenizer = Tokenizer::new().unwrap_or_else(|_| {
            // Fallback to default tokenizer if initialization fails
            Tokenizer::default()
        });
        Self { budget, tokenizer }
    }

    pub fn with_tokenizer(budget: usize, tokenizer: Tokenizer) -> Self {
        Self { budget, tokenizer }
    }

    /// Get reference to tokenizer
    pub fn tokenizer(&self) -> &Tokenizer {
        &self.tokenizer
    }

    /// Main allocation algorithm
    pub fn allocate(
        &self,
        sections: Vec<Section>,
        lens_registry: &LensRegistry,
    ) -> EngineResult<AllocationResult> {
        self.allocate_with_mode(sections, lens_registry, ExecutionMode::Exec)
    }

    pub fn allocate_with_mode(
        &self,
        mut sections: Vec<Section>,
        lens_registry: &LensRegistry,
        mode: ExecutionMode,
    ) -> EngineResult<AllocationResult> {
        for (idx, section) in sections.iter_mut().enumerate() {
            section.source_index = idx;
        }

        // Step 1: Calculate Fixed Load
        let (fixed_load, _critical_sections) = self.calculate_fixed_load(&sections)?;

        if fixed_load > self.budget {
            return Err(EngineError::BudgetExceeded {
                budget: self.budget,
                required: fixed_load,
            });
        }

        // Step 2: If everything fits, keep all sections as-is.
        let current_total: usize = sections.iter().map(|s| s.current_size).sum();
        if current_total <= self.budget {
            let mut allocated_sections = Vec::new();
            for section in sections {
                allocated_sections.push(AllocatedSection {
                    final_size: section.current_size,
                    was_compressed: false,
                    was_truncated: false,
                    was_dropped: false,
                    section,
                });
            }
            return Ok(AllocationResult {
                sections: sort_allocated_by_source(allocated_sections),
                total_size: current_total,
                budget: self.budget,
                overflow: 0,
            });
        }

        // Step 3: Compression/drop for flexible sections (if needed)
        let allocation_result = self.compress_sections(sections, lens_registry, mode)?;

        Ok(allocation_result)
    }

    /// Step 1: Calculate Fixed Load from critical sections
    fn calculate_fixed_load(&self, sections: &[Section]) -> EngineResult<(usize, Vec<usize>)> {
        let mut fixed_load = 0;
        let mut critical_indices = Vec::new();

        for (i, section) in sections.iter().enumerate() {
            if section.is_critical {
                fixed_load += section.base_size;
                critical_indices.push(i);
            }
        }

        Ok((fixed_load, critical_indices))
    }

    /// Step 3: Compress sections to fit budget
    fn compress_sections(
        &self,
        sections: Vec<Section>,
        lens_registry: &LensRegistry,
        mode: ExecutionMode,
    ) -> EngineResult<AllocationResult> {
        // Separate critical and flexible sections
        let mut critical_sections: Vec<Section> = Vec::new();
        let mut flexible_sections: Vec<Section> = Vec::new();

        for section in sections {
            if section.is_critical {
                critical_sections.push(section);
            } else {
                flexible_sections.push(section);
            }
        }

        // Always keep critical sections
        let mut allocated_sections: Vec<AllocatedSection> = critical_sections
            .into_iter()
            .map(|s| AllocatedSection {
                final_size: s.current_size,
                was_compressed: false,
                was_truncated: false,
                was_dropped: false,
                section: s,
            })
            .collect();

        let critical_total: usize = allocated_sections.iter().map(|a| a.final_size).sum();

        if flexible_sections.is_empty() {
            return Ok(AllocationResult {
                sections: allocated_sections,
                total_size: critical_total,
                budget: self.budget,
                overflow: critical_total.saturating_sub(self.budget),
            });
        }

        let flexible_total: usize = flexible_sections.iter().map(|s| s.current_size).sum();
        let current_total = critical_total + flexible_total;

        if current_total <= self.budget {
            // No compression needed, add all flexible sections
            for section in flexible_sections {
                allocated_sections.push(AllocatedSection {
                    final_size: section.current_size,
                    was_compressed: false,
                    was_truncated: false,
                    was_dropped: false,
                    section,
                });
            }

            return Ok(AllocationResult {
                sections: sort_allocated_by_source(allocated_sections),
                total_size: current_total,
                budget: self.budget,
                overflow: 0,
            });
        }

        // Sort flexible sections by (priority ASC, shrink DESC, original section order ASC)
        flexible_sections.sort_by(|a, b| {
            a.priority
                .cmp(&b.priority)
                .then_with(|| {
                    b.shrink
                        .partial_cmp(&a.shrink)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .then_with(|| a.source_index.cmp(&b.source_index))
        });

        let mut running_total = current_total;

        for mut section in flexible_sections.into_iter() {
            let mut was_compressed = false;
            let mut was_truncated = false;
            let original_size = section.current_size;

            if running_total <= self.budget {
                allocated_sections.push(AllocatedSection {
                    final_size: section.current_size,
                    was_compressed: was_compressed || section.current_size < original_size,
                    was_truncated: false,
                    was_dropped: false,
                    section,
                });
                continue;
            }

            // Try compression first
            if section.strategy.is_some() {
                let (compressed_content, compressed_size) =
                    section.apply_compression(lens_registry, &self.tokenizer, mode)?;
                let size_reduction = section.current_size.saturating_sub(compressed_size);

                if size_reduction > 0 {
                    section.content = compressed_content;
                    section.current_size = compressed_size;
                    running_total = running_total.saturating_sub(size_reduction);
                    was_compressed = true;
                }
            }

            // If still over budget, truncate deterministically from the end down to `min`.
            if running_total > self.budget && section.current_size > section.min {
                let need = running_total - self.budget;
                let reducible = section.current_size - section.min;
                let requested_reduction = std::cmp::min(need, reducible);
                if requested_reduction > 0 {
                    let target_size = section.current_size - requested_reduction;
                    let (truncated_content, truncated_size) = self.truncate_content(
                        &section.content,
                        target_size,
                        section.min,
                    );
                    if truncated_size < section.current_size {
                        section.content = truncated_content;
                        running_total = running_total.saturating_sub(
                            section.current_size.saturating_sub(truncated_size),
                        );
                        section.current_size = truncated_size;
                        was_truncated = true;
                    }
                }
            }

            // If still over budget and this section is at min, drop it.
            if running_total > self.budget && section.current_size == section.min {
                running_total = running_total.saturating_sub(section.current_size);
                allocated_sections.push(AllocatedSection {
                    final_size: 0,
                    was_compressed: was_compressed,
                    was_truncated,
                    was_dropped: true,
                    section,
                });
            } else {
                allocated_sections.push(AllocatedSection {
                    final_size: section.current_size,
                    was_compressed: was_compressed || section.current_size < original_size,
                    was_truncated,
                    was_dropped: false,
                    section,
                });
            }
        }

        let allocated_sections = sort_allocated_by_source(allocated_sections);

        let final_total: usize = allocated_sections.iter().map(|a| a.final_size).sum();

        Ok(AllocationResult {
            sections: allocated_sections,
            total_size: final_total,
            budget: self.budget,
            overflow: final_total.saturating_sub(self.budget),
        })
    }

    fn truncate_content(
        &self,
        content: &ValueNode,
        target_units: usize,
        min_units: usize,
    ) -> (ValueNode, usize) {
        let current_units = self.tokenizer.count_facet_units_in_value(content);
        if current_units <= target_units {
            return (content.clone(), current_units);
        }

        // Clamp requested bounds to a valid range for deterministic behavior.
        let target_units = target_units.min(current_units);
        let min_units = min_units.min(target_units);

        match content {
            ValueNode::String(text) => {
                let current = self.tokenizer.count_facet_units(text);
                let mut best_idx: Option<usize> = None;
                let mut best_units = current;

                for idx in text
                    .char_indices()
                    .map(|(i, _)| i)
                    .chain(std::iter::once(text.len()))
                {
                    let prefix = &text[..idx];
                    let units = self.tokenizer.count_facet_units(prefix);
                    if units > target_units {
                        break;
                    }
                    if units >= min_units {
                        best_idx = Some(idx);
                        best_units = units;
                    }
                }

                if let Some(idx) = best_idx {
                    let truncated = text[..idx].to_string();
                    (ValueNode::String(truncated), best_units)
                } else {
                    (ValueNode::String(text.clone()), current)
                }
            }
            ValueNode::List(items) => self.truncate_list(items, target_units, min_units),
            ValueNode::Map(map) => self.truncate_map(map, target_units, min_units),
            _ => {
                // Atomic non-string values cannot be partially truncated.
                (content.clone(), current_units)
            }
        }
    }

    fn truncate_list(
        &self,
        items: &[ValueNode],
        target_units: usize,
        min_units: usize,
    ) -> (ValueNode, usize) {
        let mut truncated_items = items.to_vec();
        let mut item_sizes: Vec<usize> = truncated_items
            .iter()
            .map(|item| self.tokenizer.count_facet_units_in_value(item))
            .collect();
        let mut current_units: usize = item_sizes.iter().sum();

        while current_units > target_units {
            let Some(last_idx) = truncated_items.len().checked_sub(1) else {
                break;
            };
            let last_size = item_sizes[last_idx];
            let prefix_units = current_units.saturating_sub(last_size);

            // Keep list-prefix and truncate from the end of the last item first.
            let child_min = min_units.saturating_sub(prefix_units);
            let child_target = target_units.saturating_sub(prefix_units);
            let (new_last, new_last_size) =
                self.truncate_content(&truncated_items[last_idx], child_target, child_min);

            if new_last_size < last_size {
                truncated_items[last_idx] = new_last;
                item_sizes[last_idx] = new_last_size;
                current_units = prefix_units + new_last_size;
                continue;
            }

            // If the tail cannot be reduced further, drop the tail item if min allows.
            if prefix_units >= min_units {
                truncated_items.pop();
                item_sizes.pop();
                current_units = prefix_units;
                continue;
            }

            break;
        }

        (ValueNode::List(truncated_items), current_units)
    }

    fn truncate_map(
        &self,
        map: &OrderedMap<String, ValueNode>,
        target_units: usize,
        min_units: usize,
    ) -> (ValueNode, usize) {
        let mut truncated_map = map.clone();
        let mut current_units = self.tokenizer.count_facet_units_in_value(&ValueNode::Map(
            truncated_map.clone(),
        ));
        let keys: Vec<String> = map.keys().cloned().collect();

        while current_units > target_units {
            let mut progress = false;

            for key in keys.iter().rev() {
                let Some(value) = truncated_map.get(key).cloned() else {
                    continue;
                };
                let value_units = self.tokenizer.count_facet_units_in_value(&value);
                let fixed_units = current_units.saturating_sub(value_units);
                let child_min = min_units.saturating_sub(fixed_units);
                let child_target = target_units.saturating_sub(fixed_units);
                let (new_value, new_value_units) =
                    self.truncate_content(&value, child_target, child_min);

                if new_value_units < value_units {
                    truncated_map.insert(key.clone(), new_value);
                    current_units = fixed_units + new_value_units;
                    progress = true;
                    if current_units <= target_units {
                        break;
                    }
                }
            }

            if !progress {
                break;
            }
        }

        (ValueNode::Map(truncated_map), current_units)
    }
}

fn sort_allocated_by_source(mut sections: Vec<AllocatedSection>) -> Vec<AllocatedSection> {
    sections.sort_by(|a, b| a.section.source_index.cmp(&b.section.source_index));
    sections
}
