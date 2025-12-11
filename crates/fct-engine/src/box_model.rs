// ============================================================================
// TOKEN BOX MODEL (Phase 4)
// ============================================================================

use crate::errors::{EngineError, EngineResult};
use crate::tokenizer::Tokenizer;
use fct_ast::{PipelineNode, ValueNode};
use fct_std::{LensContext, LensRegistry};
use std::collections::HashMap;

/// Represents a logical prompt section with allocation attributes
#[derive(Debug, Clone)]
pub struct Section {
    pub id: String,
    pub priority: i32,                  // lower = dropped earlier
    pub base_size: usize,               // token count after initial render
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
        let base_size = tokenizer.count_tokens_in_value(&content);
        Self {
            id,
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
    pub fn apply_compression(&self, lens_registry: &LensRegistry, tokenizer: &Tokenizer) -> EngineResult<usize> {
        if let Some(strategy) = &self.strategy {
            // Apply compression pipeline directly to content
            let mut current_value = self.content.clone();
            let ctx = LensContext {
                variables: HashMap::new(),
            };

            for lens_call in &strategy.lenses {
                let lens = lens_registry.get(&lens_call.name).ok_or_else(|| {
                    EngineError::LensExecutionFailed {
                        message: format!("Unknown compression lens: {}", lens_call.name),
                    }
                })?;

                current_value = lens
                    .execute(
                        current_value,
                        lens_call.args.clone(),
                        lens_call.kwargs.clone(),
                        &ctx,
                    )
                    .map_err(|e| EngineError::LensExecutionFailed {
                        message: format!("Compression lens '{}' failed: {}", lens_call.name, e),
                    })?;
            }

            // Calculate real token count after compression
            Ok(tokenizer.count_tokens_in_value(&current_value))
        } else {
            Ok(self.current_size)
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
        let tokenizer = Tokenizer::new()
            .unwrap_or_else(|_| {
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
        mut sections: Vec<Section>,
        lens_registry: &LensRegistry,
    ) -> EngineResult<AllocationResult> {
        // Step 1: Calculate Fixed Load
        let (fixed_load, _critical_sections) = self.calculate_fixed_load(&sections)?;

        if fixed_load > self.budget {
            return Err(EngineError::BudgetExceeded {
                budget: self.budget,
                required: fixed_load,
            });
        }

        // Step 2: Calculate current total and Free Space
        let current_total: usize = sections.iter().map(|s| s.current_size).sum();
        let free_space = self.budget - fixed_load;

        // Step 3: Expansion (only if we have space and no compression needed)
        if current_total <= self.budget && free_space > 0 && !sections.is_empty() {
            self.expand_sections(&mut sections, free_space)?;
            let expanded_total: usize = sections.iter().map(|s| s.current_size).sum();

            // Return the expanded result (budget is large enough to accommodate)
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

            // Sort final sections by ID for deterministic output
            allocated_sections.sort_by(|a, b| a.section.id.cmp(&b.section.id));

            return Ok(AllocationResult {
                sections: allocated_sections,
                total_size: expanded_total,
                budget: self.budget,
                overflow: 0,
            });
        }

        // Step 4: Compression (if needed)
        let allocation_result = self.compress_sections(sections, lens_registry)?;

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

    /// Step 3: Expand sections with free space proportionally
    fn expand_sections(&self, sections: &mut [Section], free_space: usize) -> EngineResult<()> {
        if free_space == 0 {
            return Ok(());
        }

        // Find sections that can grow
        let mut total_grow_weight = 0.0;
        let mut grow_sections = Vec::new();

        for section in sections.iter_mut() {
            if section.grow > 0.0 {
                total_grow_weight += section.grow;
                grow_sections.push(section);
            }
        }

        if total_grow_weight == 0.0 {
            return Ok(());
        }

        // Sort by ID for deterministic behavior, then distribute free space proportionally
        grow_sections.sort_by(|a, b| a.id.cmp(&b.id));
        for section in grow_sections.iter_mut() {
            let growth = (free_space as f64 * (section.grow / total_grow_weight)) as usize;
            section.current_size = section.base_size + growth;
        }

        Ok(())
    }

    /// Step 4: Compress sections to fit budget
    fn compress_sections(
        &self,
        sections: Vec<Section>,
        lens_registry: &LensRegistry,
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
                sections: allocated_sections,
                total_size: current_total,
                budget: self.budget,
                overflow: 0,
            });
        }

        let deficit = current_total - self.budget;
        let remaining_budget = self.budget - critical_total;

        // Sort flexible sections by (priority ASC, shrink DESC, id ASC) for deterministic ordering
        flexible_sections.sort_by(|a, b| {
            a.priority
                .cmp(&b.priority)
                .then_with(|| {
                    b.shrink
                        .partial_cmp(&a.shrink)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .then_with(|| a.id.cmp(&b.id))
        });

        let mut remaining_deficit = deficit;
        let mut remaining_flexible_budget = remaining_budget;

        for mut section in flexible_sections.into_iter() {
            let mut was_compressed = false;
            let original_size = section.current_size;

            if remaining_deficit == 0 {
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
                let compressed_size = section.apply_compression(lens_registry, &self.tokenizer)?;
                let size_reduction = section.current_size.saturating_sub(compressed_size);

                if size_reduction > 0 {
                    section.current_size = compressed_size;
                    remaining_deficit = remaining_deficit.saturating_sub(size_reduction);
                    remaining_flexible_budget =
                        remaining_flexible_budget.saturating_sub(size_reduction);
                    was_compressed = true;
                }
            }

            // If still over budget and we can shrink further
            if remaining_deficit > 0
                && remaining_flexible_budget > 0
                && section.current_size > section.min
            {
                let max_shrink = section.current_size - section.min;
                let actual_shrink = std::cmp::min(max_shrink, remaining_flexible_budget);
                section.current_size -= actual_shrink;
                remaining_flexible_budget -= actual_shrink;
            }

            // If still no budget left, drop the section
            if remaining_flexible_budget == 0 {
                remaining_deficit = remaining_deficit.saturating_sub(section.current_size);
                allocated_sections.push(AllocatedSection {
                    final_size: 0,
                    was_compressed: false,
                    was_truncated: false,
                    was_dropped: true,
                    section,
                });
            } else {
                allocated_sections.push(AllocatedSection {
                    final_size: section.current_size,
                    was_compressed: was_compressed || section.current_size < original_size,
                    was_truncated: section.current_size == section.min,
                    was_dropped: false,
                    section,
                });
            }
        }

        // Sort final sections by ID for deterministic output
        allocated_sections.sort_by(|a, b| a.section.id.cmp(&b.section.id));

        let final_total: usize = allocated_sections.iter().map(|a| a.final_size).sum();

        Ok(AllocationResult {
            sections: allocated_sections,
            total_size: final_total,
            budget: self.budget,
            overflow: final_total.saturating_sub(self.budget),
        })
    }
}
