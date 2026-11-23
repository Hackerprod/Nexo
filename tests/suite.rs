// Each test file keeps its own `tests/common/mod.rs` instance, as when Cargo
// compiled the files as separate test binaries. This preserves their state.
#![allow(clippy::duplicate_mod)]

include!("j16_frontend_stack.rs");

mod j14_frontend_stack {
    include!("j14_frontend_stack.rs");

    pub(super) fn invoke_measured_stack_child() {
        measured_stack_child();
    }
}

#[test]
fn measured_stack_child() {
    j14_frontend_stack::invoke_measured_stack_child();
}

#[path = "analysis_profiles_and_quotas.rs"]
mod analysis_profiles_and_quotas;
#[path = "canonical_print.rs"]
mod canonical_print;
#[path = "cli.rs"]
mod cli;
#[path = "cli_argument_and_unicode_edges.rs"]
mod cli_argument_and_unicode_edges;
#[path = "corpus.rs"]
mod corpus;
#[path = "corpus_objetivo.rs"]
mod corpus_objetivo;
#[path = "explain_and_print_output.rs"]
mod explain_and_print_output;
#[path = "f0005_b1b.rs"]
mod f0005_b1b;
#[path = "f0007.rs"]
mod f0007;
#[path = "i01_diagnostic.rs"]
mod i01_diagnostic;
#[path = "i07c_causal_obligations.rs"]
mod i07c_causal_obligations;
#[path = "i07c_deferred_constraints.rs"]
mod i07c_deferred_constraints;
#[path = "i07c_deferred_group_regressions.rs"]
mod i07c_deferred_group_regressions;
#[path = "i07c_deferred_metamorphic.rs"]
mod i07c_deferred_metamorphic;
#[path = "i07c_deferred_mutation_cases.rs"]
mod i07c_deferred_mutation_cases;
#[path = "i07c_deferred_twins.rs"]
mod i07c_deferred_twins;
#[path = "i07c_group_primary.rs"]
mod i07c_group_primary;
#[path = "i07c_mutation_depth.rs"]
mod i07c_mutation_depth;
#[path = "i07c_mutation_regressions.rs"]
mod i07c_mutation_regressions;
#[path = "i07c_normative_literals.rs"]
mod i07c_normative_literals;
#[path = "i07c_normative_survivors.rs"]
mod i07c_normative_survivors;
#[path = "i07c_pattern_deferred.rs"]
mod i07c_pattern_deferred;
#[path = "i07c_pending_nominal_domain.rs"]
mod i07c_pending_nominal_domain;
#[path = "i07c_rd_conformidad.rs"]
mod i07c_rd_conformidad;
#[path = "i07c_s4_context_regressions.rs"]
mod i07c_s4_context_regressions;
#[path = "i07c_s5_seed_consistency.rs"]
mod i07c_s5_seed_consistency;
#[path = "i07c_s5b_selector.rs"]
mod i07c_s5b_selector;
#[path = "i07c_solver_depth.rs"]
mod i07c_solver_depth;
#[path = "i07c_solver_quota_literal.rs"]
mod i07c_solver_quota_literal;
#[path = "i07c_structural_invariants.rs"]
mod i07c_structural_invariants;
#[path = "i08.rs"]
mod i08;
#[path = "i08_campaign.rs"]
mod i08_campaign;
#[path = "i08_robustness.rs"]
mod i08_robustness;
#[path = "i09_campaign.rs"]
mod i09_campaign;
#[path = "i10b_campaign.rs"]
mod i10b_campaign;
#[path = "i11a.rs"]
mod i11a;
#[path = "j01_profile.rs"]
mod j01_profile;
#[path = "j02_option.rs"]
mod j02_option;
#[path = "j03_match.rs"]
mod j03_match;
#[path = "j04_match_fuzz.rs"]
mod j04_match_fuzz;
#[path = "j04_match_ownership.rs"]
mod j04_match_ownership;
#[path = "j04_match_semantics.rs"]
mod j04_match_semantics;
#[path = "j05_match_fuzz.rs"]
mod j05_match_fuzz;
#[path = "j05_match_guards.rs"]
mod j05_match_guards;
#[path = "j06_j09_validation.rs"]
mod j06_j09_validation;
#[path = "j06_names.rs"]
mod j06_names;
#[path = "j07_retention.rs"]
mod j07_retention;
#[path = "j08_receiver.rs"]
mod j08_receiver;
#[path = "j09_closure_fuzz.rs"]
mod j09_closure_fuzz;
#[path = "j09_effects.rs"]
mod j09_effects;
#[path = "j10_generator.rs"]
mod j10_generator;
#[path = "j10_j13_fuzz.rs"]
mod j10_j13_fuzz;
#[path = "j10_j13_regressions.rs"]
mod j10_j13_regressions;
#[path = "j11_generator.rs"]
mod j11_generator;
#[path = "j12_generator.rs"]
mod j12_generator;
#[path = "j13_iteration.rs"]
mod j13_iteration;
#[path = "j14_accounting.rs"]
mod j14_accounting;
#[path = "j14_diagnostic_publication.rs"]
mod j14_diagnostic_publication;
#[path = "j14_dispatch_guard.rs"]
mod j14_dispatch_guard;
#[path = "j14_edge_cleanup.rs"]
mod j14_edge_cleanup;
#[path = "j14_match_inference_edges.rs"]
mod j14_match_inference_edges;
#[path = "j14_s3_values.rs"]
mod j14_s3_values;
#[path = "j14_s4_depth.rs"]
mod j14_s4_depth;
#[path = "j14_semantic_boundaries.rs"]
mod j14_semantic_boundaries;
#[path = "j15_cli.rs"]
mod j15_cli;
#[path = "j16_diagnostics.rs"]
mod j16_diagnostics;
#[path = "j16_for_scope.rs"]
mod j16_for_scope;
#[path = "j16_integrated.rs"]
mod j16_integrated;
#[path = "j16_question_edge.rs"]
mod j16_question_edge;
#[path = "mock_env_json_and_read_boundaries.rs"]
mod mock_env_json_and_read_boundaries;
#[path = "mock_env_records_and_failures.rs"]
mod mock_env_records_and_failures;
#[path = "mock_oracle.rs"]
mod mock_oracle;
#[path = "parenthesized_places.rs"]
mod parenthesized_places;
#[path = "run_argv_dash_value_semantics.rs"]
mod run_argv_dash_value_semantics;
#[path = "run_argv_matrix.rs"]
mod run_argv_matrix;
#[path = "run_cli_diagnostics_and_cells.rs"]
mod run_cli_diagnostics_and_cells;
#[path = "runtime_profiles_and_resource_limits.rs"]
mod runtime_profiles_and_resource_limits;
