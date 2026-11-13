#!/usr/bin/env python3
"""Mutación de comportamiento I11a en copia del árbol de trabajo, una a una."""

import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
COPY = ROOT / ".i11a-mutation-copy"
OUTPUT = ROOT / "docs/validation/F0-0002-I11a-mutaciones.json"


def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


# (id, path, old, new, occurrence, test target, exact test)
MUTATIONS = [
    ("view_verify_shape", "src/ir/verify.rs", "matches!(intrinsic, Intrinsic::Inspect | Intrinsic::MockReadRecords)", "*intrinsic == Intrinsic::Inspect", 0, "cli", "cli_mock_read_temporary_has_exact_ok_and_err_json"),
    ("view_runtime_restore", "src/interp/machine.rs", "matches!(intrinsic, Intrinsic::Inspect | Intrinsic::MockReadRecords)", "*intrinsic == Intrinsic::Inspect", 0, "cli", "cli_mock_read_temporary_has_exact_ok_and_err_json"),
    ("read_failure_dispose", "src/interp/machine.rs", "if *intrinsic == Intrinsic::MockReadRecords {\n                                        self.dispose_intrinsic_resources(&mut values)?;\n                                    }", "if false {}", 0, "i11a", "read_batch_quota_failure_closes_temporary_resource"),
    ("reply_err_event", "src/interp/machine.rs", "Err(error) => format!(\"reply:{status}:err:{error}\"),", "Err(error) => format!(\"reply:{status}:ok\"),", 0, "i11a", "request_reply_queues_preserve_fifo_errors_and_ordered_observations"),
    ("reply_no_pop", "src/interp/machine.rs", "self.mock_env.reply.pop_front();", "let _ = &self.mock_env.reply;", 0, "i11a", "request_reply_queues_preserve_fifo_errors_and_ordered_observations"),
    ("scalar_err_flag", "src/interp/machine.rs", "Self::Read(Err(n)) | Self::Scalar(Err(n)) => (false, Value::Int(n)),", "Self::Read(Err(n)) | Self::Scalar(Err(n)) => (true, Value::Int(n)),", 0, "i11a", "request_reply_queues_preserve_fifo_errors_and_ordered_observations"),
    ("request_back", "src/interp/machine.rs", "self.mock_env.request.front().cloned().unwrap_or(Err(-9000));", "self.mock_env.request.back().cloned().unwrap_or(Err(-9000));", 0, "i11a", "request_reply_queues_preserve_fifo_errors_and_ordered_observations"),
    ("read_empty_code", "src/interp/machine.rs", "                    .read_records\n                    .front()\n                    .cloned()\n                    .unwrap_or(Err(-9000));", "                    .read_records\n                    .front()\n                    .cloned()\n                    .unwrap_or(Err(-9001));", 0, "i11a", "exhausted_mock_queues_return_the_contract_error"),
    ("write_empty_code", "src/interp/machine.rs", "                    .write_i64\n                    .front()\n                    .cloned()\n                    .unwrap_or(Err(-9000));", "                    .write_i64\n                    .front()\n                    .cloned()\n                    .unwrap_or(Err(-9001));", 0, "i11a", "exhausted_mock_queues_return_the_contract_error"),
    ("batch_order", "src/interp/machine.rs", "batch.push((super::observation::Channel::Stdout, line, String::new()));", "batch.insert(0, (super::observation::Channel::Stdout, line, String::new()));", 0, "cli", "cli_json_preserves_err_types_channels_and_exit_status"),
    ("batch_gt_ge", "src/interp/observation.rs", "if proposed > self.limit {\n            return Err(proposed);\n        }\n        self.bytes += added;", "if proposed >= self.limit {\n            return Err(proposed);\n        }\n        self.bytes += added;", 0, "i11a", "observation_limits_are_atomic_and_include_close_reservations"),
    ("batch_no_reserved", "src/interp/observation.rs", ".checked_add(self.reserved)\n            .and_then(|n| n.checked_add(added))", ".checked_add(0)\n            .and_then(|n| n.checked_add(added))", 0, "i11a", "observation_limits_are_atomic_and_include_close_reservations"),
    ("batch_release_cells", "src/interp/machine.rs", "self.cells = self.cells.saturating_sub(cells);\n            self.observation_fault(span, observed);", "self.observation_fault(span, observed);", 0, "lib", "interp::machine::tests::rejected_mock_batch_releases_reserved_result_cells"),
    ("read_cells_x1", "src/interp/machine.rs", "Ok(records) => 2u64.saturating_add((records.len() as u64).saturating_mul(2)),", "Ok(records) => 2u64.saturating_add((records.len() as u64).saturating_mul(1)),", 0, "i11a", "reduced_cell_profile_keeps_read_batch_atomic_and_cleans_open_resource"),
    ("array_cells", "src/interp/machine.rs", "let cells = 1 + (values.len() as u64) * 2;", "let cells = 1 + (values.len() as u64);", 0, "i11a", "reduced_cell_profile_covers_array_request_and_index_costs"),
    ("request_cells_1", "src/interp/machine.rs", "(MockReplyValue::Scalar(reply), event, None, 2, 2)", "(MockReplyValue::Scalar(reply), event, None, 1, 2)", 0, "i11a", "reduced_cell_profile_covers_array_request_and_index_costs"),
    ("read_work_plus1", "src/interp/machine.rs", "Ok(records) => 2u64.saturating_add((records.len() as u64).saturating_mul(2)),", "Ok(records) => 3u64.saturating_add((records.len() as u64).saturating_mul(2)),", 1, "i11a", "mock_primitive_work_costs_use_exact_normal_limit"),
    ("write_work_const", "src/interp/machine.rs", "let work = 2 + u64::from(stdout.is_some());", "let work = 2;", 0, "i11a", "mock_primitive_work_costs_use_exact_normal_limit"),
    ("indexvalue_own_1", "src/interp/machine.rs", "let own = u64::from(matches!(taken, TempValue::Own(_)));", "let own = 1;", 0, "i11a", "reduced_cell_profile_covers_array_request_and_index_costs"),
    ("indexvalue_own_0", "src/interp/machine.rs", "let own = u64::from(matches!(taken, TempValue::Own(_)));", "let own = 0;", 0, "lib", "interp::machine::tests::repeated_index_value_releases_unselected_array_cells"),
    ("indexvalue_ok_branch_plus1", "src/interp/machine.rs", "self.cells = self.cells.saturating_sub(size + own - selected.cells());", "self.cells = self.cells.saturating_sub(size + 1 - selected.cells());", 0, "i11a", "reduced_cell_profile_covers_array_request_and_index_costs"),
    ("loan_index_always_dec", "src/interp/machine.rs", "if matches!(taken, TempValue::Own(_)) {\n                            self.cells = self.cells.saturating_sub(1);\n                        }\n                        path.push(*index);", "{\n                            self.cells = self.cells.saturating_sub(1);\n                        }\n                        path.push(*index);", 0, "i11a", "reduced_cell_profile_covers_array_request_and_index_costs"),
    ("read_index_always_dec", "src/interp/machine.rs", "if matches!(taken, TempValue::Own(_)) {\n                                self.cells = self.cells.saturating_sub(1);\n                            }\n                            path.push(*n);", "{\n                                self.cells = self.cells.saturating_sub(1);\n                            }\n                            path.push(*n);", 0, "i11a", "reduced_cell_profile_covers_array_request_and_index_costs"),
    ("env_depth_17", "src/interp/mock_env.rs", "if self.depth >= 16 {", "if self.depth >= 17 {", 0, "i11a", "mock_environment_parser_enforces_exact_size_and_profile_boundaries"),
    ("env_depth_15", "src/interp/mock_env.rs", "if self.depth >= 16 {", "if self.depth >= 15 {", 0, "i11a", "mock_environment_parser_enforces_exact_size_and_profile_boundaries"),
    ("env_leading_zero", "src/interp/mock_env.rs", "|| (s.starts_with('0') && s.len() > 1)\n", "\n", 0, "cli", "cli_mock_environment_option_is_run_only_and_reports_exact_schema_reason"),
    ("env_neg_leading_zero", "src/interp/mock_env.rs", "|| (s.starts_with(\"-0\") && s.len() > 2)\n", "\n", 0, "cli", "cli_mock_environment_option_is_run_only_and_reports_exact_schema_reason"),
    ("env_size_limit", "src/interp/mock_env.rs", "bytes.len() > 1_048_576", "bytes.len() > 1_048_577", 0, "i11a", "mock_environment_parser_enforces_exact_size_and_profile_boundaries"),
    ("env_version_value", "src/interp/mock_env.rs", "Json::Number(ref n) if n == \"1\"", "Json::Number(ref n) if n == \"1\" || n == \"1.0\"", 0, "cli", "cli_mock_environment_option_is_run_only_and_reports_exact_schema_reason"),
    ("env_unit_null", "src/interp/mock_env.rs", "matches!(v, Json::Null)", "matches!(v, Json::Null | Json::Bool)", 0, "cli", "cli_mock_environment_option_is_run_only_and_reports_exact_schema_reason"),
    ("effects_request_io", "src/ir/effects.rs", "Print | Inspect | MockOpen | MockBlockingOpen | MockWriteI64 | MockRequest | MockReply => {", "MockRequest => (0, 0, \"intrinsic.call\"),\n        Print | Inspect | MockOpen | MockBlockingOpen | MockWriteI64 | MockReply => {", 0, "cli", "cli_request_requires_io_effect_and_reply_effect_name_is_specific"),
    ("effects_name_reply", "src/ir/effects.rs", "super::model::Intrinsic::MockReply => \"mock_reply\",", "super::model::Intrinsic::MockReply => \"mock_request\",", 0, "cli", "cli_request_requires_io_effect_and_reply_effect_name_is_specific"),
    ("cli_mock_only_run", "src/main.rs", "\"--mock-env\"\n                    if command == \"run\"\n                        && !seen_mock", "\"--mock-env\"\n                    if !seen_mock", 0, "cli", "cli_mock_environment_option_is_run_only_and_reports_exact_schema_reason"),
    ("cli_json_exit_err", "src/main.rs", "nexo::interp::Outcome::Err(_) | nexo::interp::Outcome::SimulatorFault => {\n                ExitCode::FAILURE\n            }\n            nexo::interp::Outcome::InternalError(_) => ExitCode::from(2),", "nexo::interp::Outcome::SimulatorFault => {\n                ExitCode::FAILURE\n            }\n            nexo::interp::Outcome::InternalError(_) => ExitCode::from(2),", 0, "cli", "cli_json_preserves_err_types_channels_and_exit_status"),
    ("cli_json_err_quote", "src/main.rs", "format!(r#\"{{\"kind\":\"err\",\"error\":\"{error}\"}}\"#)", "format!(r#\"{{\"kind\":\"err\",\"error\":{error}}}\"#)", 0, "cli", "cli_json_preserves_err_types_channels_and_exit_status"),
    ("cli_json_channel", "src/main.rs", "if *c == nexo::interp::observation::Channel::Stdout {\n                    \"stdout\"", "if *c == nexo::interp::observation::Channel::Event {\n                    \"stdout\"", 0, "cli", "cli_json_preserves_err_types_channels_and_exit_status"),
    ("cli_env_too_large", "src/main.rs", "Err(InputError::TooLarge { .. }) => {\n                    return driver_error(Some(&profile_id), \"mock_env_too_large\", format == \"json\");", "Err(InputError::TooLarge { .. }) => {\n                    return driver_error(Some(&profile_id), \"mock_env_unreadable\", format == \"json\");", 0, "cli", "cli_reports_mock_environment_size_limit_reason"),
    ("cli_schema_reason", "src/main.rs", "return driver_error(Some(&profile_id), \"invalid_mock_schema\", format == \"json\");", "return driver_error(Some(&profile_id), \"invalid_mock_json\", format == \"json\");", 0, "cli", "cli_mock_environment_option_is_run_only_and_reports_exact_schema_reason"),
    ("profile_flow_global", "src/interp/machine.rs", "\"f0-0002-b-test-flow-global-32-v2\" => Self::FlowCompilation32,", "\"f0-0002-b-test-flow-global-32-v2\" => Self::FlowFunction32,", 0, "i11a", "mock_environment_parser_enforces_exact_size_and_profile_boundaries"),
    ("profile_types32_limit", "src/ir/lower.rs", "type_solver_steps: 32,", "type_solver_steps: 33,", 0, "runtime_profiles_and_resource_limits", "reduced_types32_profile_reports_the_exact_source_budget_boundary"),
    ("n_effects_write_io", "src/ir/effects.rs", "Print | Inspect | MockOpen | MockBlockingOpen | MockWriteI64 | MockRequest | MockReply => {\n            (IO, 0, \"intrinsic.call\")\n        }", "MockWriteI64 => (0, 0, \"intrinsic.call\"),\n        Print | Inspect | MockOpen | MockBlockingOpen | MockRequest | MockReply => {\n            (IO, 0, \"intrinsic.call\")\n        }", 0, "cli", "cli_mock_write_requires_io_effect"),
    ("n_write_no_pop", "src/interp/machine.rs", "self.mock_env.write_i64.pop_front();", "let _ = &self.mock_env.write_i64;", 0, "i11a", "mock_write_queue_consumes_one_reply_per_call"),
    ("n_read_pop_clear", "src/interp/machine.rs", "self.mock_env.read_records.pop_front();", "self.mock_env.read_records.clear();", 0, "i11a", "mock_read_queue_consumes_one_reply_per_call"),
    ("n_write_event_number", "src/interp/machine.rs", "Ok(()) => format!(\"write_i64:{number}:ok\"),", "Ok(()) => format!(\"write_i64:{}:ok\", number.abs()),", 0, "i11a", "mock_write_event_preserves_negative_decimal_value"),
    ("n_read_event_ok_len", "src/interp/machine.rs", "Ok(records) => format!(\"read_records:{id}:ok:{}\", records.len()),", "Ok(records) => format!(\"read_records:{id}:ok:{}\", records.len().min(9)),", 0, "i11a", "mock_read_ok_event_reports_all_records"),
    ("n_read_err_cells3", "src/interp/machine.rs", "Err(_) => 2,", "Err(_) => 3,", 0, "mock_oracle", "mock_oracle_runtime_contract"),
    ("n_read_err_work3", "src/interp/machine.rs", "Err(_) => 2,", "Err(_) => 3,", 1, "mock_oracle", "mock_oracle_runtime_contract"),
    ("n_env_tab_ws", "src/interp/mock_env.rs", "matches!(b, b' ' | b'\\n' | b'\\r' | b'\\t')", "matches!(b, b' ' | b'\\n' | b'\\r')", 0, "i11a", "mock_json_accepts_tab_as_whitespace"),
    ("n_env_ctrl_boundary", "src/interp/mock_env.rs", "if b < 0x20 {", "if b < 0x1f {", 0, "i11a", "mock_json_rejects_raw_0x1f_inside_string"),
    ("n_env_exp_upper", "src/interp/mock_env.rs", "matches!(b, b'e' | b'E')", "matches!(b, b'e')", 0, "i11a", "mock_json_uppercase_exponent_is_a_schema_error"),
    ("n_env_leave_empty_array", "src/interp/mock_env.rs", "if self.take(b']') {\n            self.leave();\n            return Ok(Json::Array(items));", "if self.take(b']') {\n            return Ok(Json::Array(items));", 0, "i11a", "mock_json_empty_arrays_restore_nesting_depth"),
    ("n_main_dup_mock", "src/main.rs", "&& !seen_mock\n", "", 0, "cli", "cli_rejects_repeated_mock_environment_option"),
    ("n_verify_peek_pass1", "src/ir/verify.rs", "let first_is_read =\n                        matches!(intrinsic, Intrinsic::Inspect | Intrinsic::MockReadRecords)", "let first_is_read =\n                        matches!(intrinsic, Intrinsic::Inspect)", 0, "cli", "cli_mock_read_temporary_has_exact_ok_and_err_json"),
    ("n_verify_peek_pass2", "src/ir/verify.rs", "let owned_resource_read =\n                            matches!(intrinsic, Intrinsic::Inspect | Intrinsic::MockReadRecords)", "let owned_resource_read =\n                            matches!(intrinsic, Intrinsic::Inspect)", 0, "i11a", "mock_primitive_work_costs_use_exact_normal_limit"),
    ("read_event_count", "src/interp/machine.rs", "Ok(records) => format!(\"read_records:{id}:ok:{}\", records.len()),", "Ok(records) => format!(\"read_records:{id}:ok:{}\", records.len() + 1),", 0, "mock_oracle", "mock_oracle_runtime_contract"),
    ("mockio_prefix", "src/interp/machine.rs", "\"mock-io:\".to_owned(),", "\"mock-resource:\".to_owned(),", 0, "mock_oracle", "mock_oracle_runtime_contract"),
    ("write_stdout_on_err", "src/interp/machine.rs", "let stdout = reply.as_ref().ok().map(|_| number.to_string());", "let stdout = Some(number.to_string());", 0, "mock_oracle", "mock_oracle_runtime_contract"),
    ("write_ok_stdout_prefix", "src/interp/machine.rs", "batch.push((super::observation::Channel::Stdout, line, String::new()));", "batch.push((super::observation::Channel::Stdout, line, \"mock-io:\".to_owned()));", 0, "i11a", "write_ok_and_err_match_literal_json_contract_data"),
    ("env_neg_zero", "src/interp/mock_env.rs", "|| s == \"-0\"\n", "\n", 0, "mock_oracle", "mock_oracle_parser_contract"),
    ("env_five_fields", "src/interp/mock_env.rs", "if fields.len() != 5 {", "if fields.len() < 5 {", 0, "mock_oracle", "mock_oracle_parser_contract"),
    ("env_result_arity", "src/interp/mock_env.rs", "if fields.len() != 1 {", "if fields.is_empty() {", 0, "mock_oracle", "mock_oracle_parser_contract"),
    ("effects_read_alloc", "src/ir/effects.rs", "MockReadRecords => (ALLOC | IO, 0, \"intrinsic.call\"),", "MockReadRecords => (IO, 0, \"intrinsic.call\"),", 0, "i11a", "read_records_is_allocating_and_the_removed_gate_has_no_runtime_branch"),
    ("lower_reply_arity", "src/ir/lower.rs", "Intrinsic::MockReadRecords | Intrinsic::MockWriteI64 | Intrinsic::MockReply => {\n                        1\n                    }", "Intrinsic::MockReadRecords | Intrinsic::MockWriteI64 => {\n                        1\n                    }\n                    Intrinsic::MockReply => 0,", 0, "cli", "cli_request_requires_io_effect_and_reply_effect_name_is_specific"),
    ("lower_request_arity", "src/ir/lower.rs", "Intrinsic::MockRequest => 0,", "Intrinsic::MockRequest => 1,", 0, "cli", "cli_request_requires_io_effect_and_reply_effect_name_is_specific"),
    ("n_request_no_pop", "src/interp/machine.rs", "self.mock_env.request.pop_front();", "let _ = &self.mock_env.request;", 0, "i11a", "request_reply_queues_preserve_fifo_errors_and_ordered_observations"),
    ("n_reply_cells_3", "src/interp/machine.rs", "(MockReplyValue::Unit(reply), event, None, 2, 2)", "(MockReplyValue::Unit(reply), event, None, 3, 2)", 0, "lib", "interp::machine::tests::write_request_and_reply_each_reserve_two_result_cells"),
    ("n_write_cells_1", "src/interp/machine.rs", "(MockReplyValue::Unit(reply), event, stdout, 2, work)", "(MockReplyValue::Unit(reply), event, stdout, 1, work)", 0, "lib", "interp::machine::tests::write_request_and_reply_each_reserve_two_result_cells"),
    ("n_obs_prefix_len", "src/interp/observation.rs", "prefix.len() + text.len() + 1).map_err(|_| u64::MAX)?;\n                sum.checked_add(n)", "text.len() + 1).map_err(|_| u64::MAX)?;\n                sum.checked_add(n)", 0, "i11a", "observation_limits_are_atomic_and_include_close_reservations"),
    ("n_effects_reply_io", "src/ir/effects.rs", "Print | Inspect | MockOpen | MockBlockingOpen | MockWriteI64 | MockRequest | MockReply => {\n            (IO, 0, \"intrinsic.call\")\n        }", "MockReply => (0, 0, \"intrinsic.call\"),\n        Print | Inspect | MockOpen | MockBlockingOpen | MockWriteI64 | MockRequest => {\n            (IO, 0, \"intrinsic.call\")\n        }", 0, "cli", "cli_request_requires_io_effect_and_reply_effect_name_is_specific"),
    ("n_env_empty_object_leave", "src/interp/mock_env.rs", "if self.take(b'}') {\n            self.leave();\n            return Ok(Json::Object(fields));", "if self.take(b'}') {\n            return Ok(Json::Object(fields));", 0, "mock_oracle", "mock_oracle_parser_contract"),
    ("n_env_nonempty_object_leave", "src/interp/mock_env.rs", "        self.leave();\n        Ok(Json::Object(fields))", "        Ok(Json::Object(fields))", 0, "mock_oracle", "mock_oracle_parser_contract"),
    ("n_env_nonempty_array_leave", "src/interp/mock_env.rs", "        self.leave();\n        Ok(Json::Array(items))", "        Ok(Json::Array(items))", 0, "mock_oracle", "mock_oracle_parser_contract"),
    ("p_depth17", "src/interp/mock_env.rs", "if self.depth >= 16 {", "if self.depth >= 17 {", 0, "mock_oracle", "mock_oracle_parser_contract"),
    ("p_size_ge", "src/interp/mock_env.rs", "bytes.len() > 1_048_576", "bytes.len() >= 1_048_576", 0, "mock_oracle", "mock_oracle_parser_contract"),
    ("p_neg0", "src/interp/mock_env.rs", "|| s == \"-0\"\n", "\n", 0, "mock_oracle", "mock_oracle_parser_contract"),
    ("p_len5", "src/interp/mock_env.rs", "if fields.len() != 5 {", "if fields.len() < 5 {", 0, "mock_oracle", "mock_oracle_parser_contract"),
    ("p_unit_any", "src/interp/mock_env.rs", "matches!(v, Json::Null)", "matches!(v, Json::Null | Json::Bool)", 0, "cli", "cli_mock_environment_option_is_run_only_and_reports_exact_schema_reason"),
    ("p_surrogate", "src/interp/mock_env.rs", "(0xDC00..=0xDFFF).contains(&second)", "(0xDC00..=0xDFFE).contains(&second)", 0, "mock_oracle", "mock_oracle_parser_contract"),
    ("p_request_event", "src/interp/machine.rs", "format!(\"request:ok:{value}\")", "format!(\"request:ok:{}\", value.abs())", 0, "mock_oracle", "mock_oracle_runtime_contract"),
    ("p_reply_err_event", "src/interp/machine.rs", "Err(error) => format!(\"reply:{status}:err:{error}\"),", "Err(error) => format!(\"reply:{status}:ok\"),", 0, "i11a", "request_reply_queues_preserve_fifo_errors_and_ordered_observations"),
    ("p_read_ok_cells", "src/interp/machine.rs", "Ok(records) => 2u64.saturating_add((records.len() as u64).saturating_mul(2)),", "Ok(records) => 1u64.saturating_add((records.len() as u64).saturating_mul(2)),", 0, "i11a", "reduced_cell_profile_keeps_read_batch_atomic_and_cleans_open_resource"),
    ("p_read_ok_work", "src/interp/machine.rs", "Ok(records) => 2u64.saturating_add((records.len() as u64).saturating_mul(2)),", "Ok(records) => 3u64.saturating_add((records.len() as u64).saturating_mul(2)),", 1, "mock_oracle", "mock_oracle_runtime_contract"),
    ("p_write_work", "src/interp/machine.rs", "let work = 2 + u64::from(stdout.is_some());", "let work = 2;", 0, "mock_oracle", "mock_oracle_runtime_contract"),
    ("p_array_cells", "src/interp/machine.rs", "let cells = 1 + (values.len() as u64) * 2;", "let cells = (values.len() as u64) * 2;", 0, "mock_oracle", "mock_oracle_runtime_contract"),
    ("p_err_stdout", "src/interp/machine.rs", "let stdout = reply.as_ref().ok().map(|_| number.to_string());", "let stdout = Some(number.to_string());", 0, "mock_oracle", "mock_oracle_runtime_contract"),
    ("cli_unknown_profile", "src/main.rs", "if nexo::ir::ReducedProfile::from_id(&profile_id).is_none() {", "if false {", 0, "cli", "cli_unknown_profile_precedes_source_read_and_json_format_is_structured"),
    ("cli_run_entry", "src/main.rs", "compile_scalar_with_profile(file, &source, profile, command == \"run\")", "compile_scalar_with_profile(file, &source, profile, false)", 0, "cli", "cli_check_and_run_use_official_b_pipeline"),
    ("cli_prefix_text", "src/main.rs", "eprintln!(\"{}{}\", observation.prefix, observation.text)", "eprintln!(\"mock-resource:{}\", observation.text)", 0, "cli", "cli_mock_primitive_is_supported_after_i11a"),
    ("explain_availability", "src/diagnostic.rs", "availability: if *id == \"implementation_limit\" {", "availability: if *id == \"implementation_limit_x\" {", 0, "lib", "diagnostic::tests::b_feature_catalog_excludes_legacy_and_validates_limit_scope"),
    ("explain_example_profile", "src/diagnostic.rs", "example_profile_id: (*id != \"implementation_limit\").then(|| PROFILE_ID.into()),", "example_profile_id: Some(PROFILE_ID.into()),", 0, "lib", "diagnostic::tests::b_feature_catalog_excludes_legacy_and_validates_limit_scope"),
    ("into_record_ok_false", "src/interp/machine.rs", "Ok(n) => Value::Result {\n                            ok: true,", "Ok(n) => Value::Result {\n                            ok: false,", 0, "mock_oracle", "mock_oracle_runtime_contract"),
    ("into_record_err_true", "src/interp/machine.rs", "Err(n) => Value::Result {\n                            ok: false,", "Err(n) => Value::Result {\n                            ok: true,", 0, "mock_oracle", "mock_oracle_runtime_contract"),
    ("into_scalar_ok_false", "src/interp/machine.rs", "Self::Scalar(Ok(n)) => (true, Value::Int(n)),", "Self::Scalar(Ok(n)) => (false, Value::Int(n)),", 0, "mock_oracle", "mock_oracle_runtime_contract"),
    ("into_unit_ok_false", "src/interp/machine.rs", "Self::Unit(Ok(())) => (true, Value::Unit),", "Self::Unit(Ok(())) => (false, Value::Unit),", 0, "i11a", "write_ok_and_err_match_literal_json_contract_data"),
    ("into_unit_err_true", "src/interp/machine.rs", "Self::Unit(Err(n)) => (false, Value::Int(n)),", "Self::Unit(Err(n)) => (true, Value::Int(n)),", 0, "i11a", "write_ok_and_err_match_literal_json_contract_data"),
    ("read_queue_back", "src/interp/machine.rs", ".read_records\n                    .front()\n                    .cloned()\n                    .unwrap_or(Err(-9000));", ".read_records\n                    .back()\n                    .cloned()\n                    .unwrap_or(Err(-9000));", 0, "mock_oracle", "mock_oracle_runtime_contract"),
    ("write_queue_back", "src/interp/machine.rs", ".write_i64\n                    .front()\n                    .cloned()\n                    .unwrap_or(Err(-9000));", ".write_i64\n                    .back()\n                    .cloned()\n                    .unwrap_or(Err(-9000));", 0, "mock_oracle", "mock_oracle_runtime_contract"),
    ("reply_queue_back", "src/interp/machine.rs", "self.mock_env.reply.front().cloned().unwrap_or(Err(-9000));", "self.mock_env.reply.back().cloned().unwrap_or(Err(-9000));", 0, "mock_oracle", "mock_oracle_runtime_contract"),
    ("request_empty_code", "src/interp/machine.rs", "self.mock_env.request.front().cloned().unwrap_or(Err(-9000));", "self.mock_env.request.front().cloned().unwrap_or(Err(-9001));", 0, "mock_oracle", "mock_oracle_runtime_contract"),
    ("reply_empty_code", "src/interp/machine.rs", "self.mock_env.reply.front().cloned().unwrap_or(Err(-9000));", "self.mock_env.reply.front().cloned().unwrap_or(Err(-9001));", 0, "mock_oracle", "mock_oracle_runtime_contract"),
    ("request_work_3", "src/interp/machine.rs", "(MockReplyValue::Scalar(reply), event, None, 2, 2)", "(MockReplyValue::Scalar(reply), event, None, 2, 3)", 0, "mock_oracle", "mock_oracle_runtime_contract"),
    ("reply_work_3", "src/interp/machine.rs", "(MockReplyValue::Unit(reply), event, None, 2, 2)", "(MockReplyValue::Unit(reply), event, None, 2, 3)", 0, "mock_oracle", "mock_oracle_runtime_contract"),
    ("request_cells_3", "src/interp/machine.rs", "(MockReplyValue::Scalar(reply), event, None, 2, 2)", "(MockReplyValue::Scalar(reply), event, None, 3, 2)", 0, "lib", "interp::machine::tests::write_request_and_reply_each_reserve_two_result_cells"),
    ("reply_cells_1", "src/interp/machine.rs", "(MockReplyValue::Unit(reply), event, None, 2, 2)", "(MockReplyValue::Unit(reply), event, None, 1, 2)", 0, "lib", "interp::machine::tests::write_request_and_reply_each_reserve_two_result_cells"),
    ("work_read_ok_mul1", "src/interp/machine.rs", "2u64.saturating_add((records.len() as u64).saturating_mul(2)),", "2u64.saturating_add((records.len() as u64).saturating_mul(1)),", 1, "mock_oracle", "mock_oracle_runtime_contract"),
    ("work_read_ok_mul3", "src/interp/machine.rs", "2u64.saturating_add((records.len() as u64).saturating_mul(2)),", "2u64.saturating_add((records.len() as u64).saturating_mul(3)),", 1, "mock_oracle", "mock_oracle_runtime_contract"),
    ("work_read_ok_base1", "src/interp/machine.rs", "2u64.saturating_add((records.len() as u64).saturating_mul(2)),", "1u64.saturating_add((records.len() as u64).saturating_mul(2)),", 1, "mock_oracle", "mock_oracle_runtime_contract"),
    ("work_read_err_1", "src/interp/machine.rs", "Err(_) => 2,", "Err(_) => 1,", 1, "mock_oracle", "mock_oracle_runtime_contract"),
    ("cells_read_err_1", "src/interp/machine.rs", "Err(_) => 2,", "Err(_) => 1,", 0, "mock_oracle", "mock_oracle_runtime_contract"),
    ("cells_read_ok_base0", "src/interp/machine.rs", "2u64.saturating_add((records.len() as u64).saturating_mul(2)),", "0u64.saturating_add((records.len() as u64).saturating_mul(2)),", 0, "i11a", "reduced_cell_profile_keeps_read_batch_atomic_and_cleans_open_resource"),
    ("cells_read_ok_mul1", "src/interp/machine.rs", "2u64.saturating_add((records.len() as u64).saturating_mul(2)),", "2u64.saturating_add((records.len() as u64).saturating_mul(1)),", 0, "i11a", "reduced_cell_profile_keeps_read_batch_atomic_and_cleans_open_resource"),
    ("cells_array_base2", "src/interp/machine.rs", "let cells = 1 + (values.len() as u64) * 2;", "let cells = 2 + (values.len() as u64) * 2;", 0, "mock_oracle", "mock_oracle_runtime_contract"),
    ("env_size_lt", "src/interp/mock_env.rs", "bytes.len() > 1_048_576", "bytes.len() > 1_048_575", 0, "mock_oracle", "mock_oracle_parser_contract"),
    ("env_depth_gt", "src/interp/mock_env.rs", "if self.depth >= 16 {", "if self.depth > 16 {", 0, "mock_oracle", "mock_oracle_parser_contract"),
    ("env_hex_radix15", "src/interp/mock_env.rs", "value = value * 16\n            + chars", "value = value * 15\n            + chars", 0, "mock_oracle", "mock_oracle_parser_contract"),
    ("env_hex_digits3", "src/interp/mock_env.rs", "for _ in 0..4 {", "for _ in 0..3 {", 0, "mock_oracle", "mock_oracle_parser_contract"),
    ("env_exp_no_minus", "src/interp/mock_env.rs", "matches!(b, b'+' | b'-')", "b == b'+'", 0, "mock_oracle", "mock_oracle_parser_contract"),
    ("env_control_includes_space", "src/interp/mock_env.rs", "if b < 0x20 {", "if b <= 0x20 {", 0, "mock_oracle", "mock_oracle_parser_contract"),
    ("env_version_numeric_eq", "src/interp/mock_env.rs", "Json::Number(ref n) if n == \"1\"", "Json::Number(ref n) if n.parse::<f64>() == Ok(1.0)", 0, "mock_oracle", "mock_oracle_parser_contract"),
    ("obs_no_lf", "src/interp/observation.rs", "prefix.len() + text.len() + 1).map_err(|_| u64::MAX)?;", "prefix.len() + text.len()).map_err(|_| u64::MAX)?;", 1, "mock_oracle", "mock_oracle_runtime_contract"),
    ("obs_add_bytes_subtract", "src/interp/observation.rs", "self.bytes += added;", "self.bytes -= added;", 0, "mock_oracle", "mock_oracle_runtime_contract"),
    ("into_record_ok_payload", "src/interp/machine.rs", "ok: true,\n                            payload: Box::new(Value::Int(n)),", "ok: true,\n                            payload: Box::new(Value::Int(n + 1)),", 0, "mock_oracle", "mock_oracle_runtime_contract"),
    ("into_record_err_payload", "src/interp/machine.rs", "ok: false,\n                            payload: Box::new(Value::Int(n)),", "ok: false,\n                            payload: Box::new(Value::Int(n + 1)),", 0, "mock_oracle", "mock_oracle_runtime_contract"),
    ("request_error_event_ok", "src/interp/machine.rs", "Err(error) => format!(\"request:err:{error}\"),", "Err(error) => format!(\"request:ok:{error}\"),", 0, "mock_oracle", "mock_oracle_runtime_contract"),
    ("write_error_event_ok", "src/interp/machine.rs", "Err(error) => format!(\"write_i64:{number}:err:{error}\"),", "Err(error) => format!(\"write_i64:{number}:ok\"),", 0, "mock_oracle", "mock_oracle_runtime_contract"),
    ("reply_status_zero", "src/interp/machine.rs", "Ok(()) => format!(\"reply:{status}:ok\"),", "Ok(()) => format!(\"reply:0:ok\"),", 0, "mock_oracle", "mock_oracle_runtime_contract"),
]
MUTATIONS += [
    ("env_no_minus_number", "src/interp/mock_env.rs", "self.take(b'-');", "self.take(b'+');", 0, "mock_oracle", "mock_oracle_parser_contract"),
    ("env_fraction_no_digits", "src/interp/mock_env.rs", "if !self.peek().is_some_and(|b| b.is_ascii_digit()) {", "if self.peek().is_some_and(|b| b.is_ascii_digit()) {", 0, "mock_oracle", "mock_oracle_parser_contract"),
    ("env_exponent_no_digits", "src/interp/mock_env.rs", "if !self.peek().is_some_and(|b| b.is_ascii_digit()) {", "if self.peek().is_some_and(|b| b.is_ascii_digit()) {", 1, "mock_oracle", "mock_oracle_parser_contract"),
    ("env_exponent_allow_plus_zero", "src/interp/mock_env.rs", "matches!(b, b'+' | b'-')", "b == b'+'", 0, "mock_oracle", "mock_oracle_parser_contract"),
    ("cells_array_base0", "src/interp/machine.rs", "let cells = 1 + (values.len() as u64) * 2;", "let cells = (values.len() as u64) * 2;", 0, "mock_oracle", "mock_oracle_runtime_contract"),
    ("cells_array_mul3", "src/interp/machine.rs", "let cells = 1 + (values.len() as u64) * 2;", "let cells = 1 + (values.len() as u64) * 3;", 0, "mock_oracle", "mock_oracle_runtime_contract"),
    ("read_event_id_plus1", "src/interp/machine.rs", "Ok(records) => format!(\"read_records:{id}:ok:{}\", records.len()),", "Ok(records) => format!(\"read_records:{}:ok:{}\", id + 1, records.len()),", 0, "mock_oracle", "mock_oracle_runtime_contract"),
]

ESCAPE_MUTATIONS = [
    ("escape_quote_decode", r"""'"' => out.push('"'),""", r"""'"' => out.push('x'),"""),
    ("escape_backslash_decode", r"""'\\' => out.push('\\'),""", r"""'\\' => out.push('x'),"""),
    ("escape_slash_decode", r"""'/' => out.push('/'),""", r"""'/' => out.push('x'),"""),
    ("escape_backspace_decode", r"""'b' => out.push('\u{8}'),""", r"""'b' => out.push('x'),"""),
    ("escape_formfeed_decode", r"""'f' => out.push('\u{c}'),""", r"""'f' => out.push('x'),"""),
    ("escape_newline_decode", r"""'n' => out.push('\n'),""", r"""'n' => out.push('x'),"""),
    ("escape_return_decode", r"""'r' => out.push('\r'),""", r"""'r' => out.push('x'),"""),
    ("escape_tab_decode", r"""'t' => out.push('\t'),""", r"""'t' => out.push('x'),"""),
]
VAL55_MUTATIONS = [
    ("v55_string_escape_toggle", "src/interp/mock_env.rs", "escaped = !escaped && b == b'\\\\';", "escaped = b == b'\\\\';", 0, "mock_oracle", "mock_oracle_parser_contract"),
    ("v55_literal_false", "src/interp/mock_env.rs", 'self.literal(b"false")?;', 'self.literal(b"fals")?;', 0, "mock_env_records_and_failures", "mock_json_boolean_payloads_are_schema_errors"),
    ("v55_canon_plus", "src/interp/mock_env.rs", """        || !s
            .strip_prefix('-')
            .unwrap_or(s)
            .bytes()
            .all(|b| b.is_ascii_digit())""", "        || false", 0, "mock_oracle", "mock_oracle_covers_non_string_literals_and_canonical_i64_at_every_slot"),
    ("v55_ws_no_cr", "src/interp/mock_env.rs", "matches!(b, b' ' | b'\\n' | b'\\r' | b'\\t')", "matches!(b, b' ' | b'\\n' | b'\\t')", 0, "mock_oracle", "mock_oracle_parser_contract"),
    ("v55_utf8_reason", "src/interp/mock_env.rs", "let text = std::str::from_utf8(bytes).map_err(|_| MockEnvironmentError::InvalidJson)?;", "let text = std::str::from_utf8(bytes).map_err(|_| MockEnvironmentError::InvalidSchema)?;", 0, "mock_oracle", "mock_oracle_parser_contract"),
    ("v55_dup_last_only", "src/interp/mock_env.rs", "            let key = self.string()?;\n", "            let key = self.string()?;\n            if fields.last().is_some_and(|(old, _)| old == &key) {\n                return Err(MockEnvironmentError::InvalidSchema);\n            }\n", 0, "i11a", "mock_json_syntax_error_precedes_duplicate_key_schema_error"),
    ("v55_records_reversed", "src/interp/machine.rs", """                let values: Vec<_> = records
                    .into_iter()
                    .map(""", """                let values: Vec<_> = records
                    .into_iter()
                    .rev()
                    .map(""", 0, "mock_oracle", "mock_oracle_compares_indexed_read_record_values_in_order"),
    ("v55_records_drop_last", "src/interp/machine.rs", "let cells = 1 + (values.len() as u64) * 2;", "let cells = 1 + (values.len() as u64) * 2;\n                let mut values = values;\n                values.pop();", 0, "mock_oracle", "mock_oracle_compares_indexed_read_record_values_in_order"),
    ("v55_read_err_payload_plus1", "src/interp/machine.rs", "Self::Read(Err(n)) | Self::Scalar(Err(n)) => (false, Value::Int(n)),", "Self::Read(Err(n)) => (false, Value::Int(n.wrapping_add(1))),\n            Self::Scalar(Err(n)) => (false, Value::Int(n)),", 0, "i11a", "exhausted_mock_queues_return_the_contract_error"),
    ("v55_unit_err_payload_neg", "src/interp/machine.rs", "Self::Unit(Err(n)) => (false, Value::Int(n)),", "Self::Unit(Err(n)) => (false, Value::Int(n.wrapping_neg())),", 0, "i11a", "write_ok_and_err_match_literal_json_contract_data"),
    ("v55_stdout_abs", "src/interp/machine.rs", "reply.as_ref().ok().map(|_| number.to_string());", "reply.as_ref().ok().map(|_| number.unsigned_abs().to_string());", 0, "i11a", "mock_write_event_preserves_negative_decimal_value"),
    ("v55_reply_status_abs", "src/interp/machine.rs", 'Ok(()) => format!("reply:{status}:ok"),', 'Ok(()) => format!("reply:{}:ok", status.unsigned_abs()),', 0, "mock_env_records_and_failures", "mock_reply_events_preserve_negative_status_sign_on_both_results"),
    ("v55_read_err_event_abs", "src/interp/machine.rs", 'Err(error) => format!("read_records:{id}:err:{error}"),', 'Err(error) => format!("read_records:{id}:err:{}", error.unsigned_abs()),', 0, "mock_oracle", "mock_oracle_runtime_contract"),
    ("v55_write_err_event_abs", "src/interp/machine.rs", 'Err(error) => format!("write_i64:{number}:err:{error}"),', 'Err(error) => format!("write_i64:{number}:err:{}", error.unsigned_abs()),', 0, "mock_oracle", "mock_oracle_runtime_contract"),
    ("v55_request_err_event_abs", "src/interp/machine.rs", 'Err(error) => format!("request:err:{error}"),', 'Err(error) => format!("request:err:{}", error.unsigned_abs()),', 0, "mock_oracle", "mock_oracle_runtime_contract"),
    ("v55_order_work_cells", "src/interp/machine.rs", "if !self.charge_work(work, span) || !self.reserve_cells(cells, span) {", "if !self.reserve_cells(cells, span) || !self.charge_work(work, span) {", 0, "mock_oracle_ignored", "mock_oracle_work_cells_priority_probe"),
    ("v55_read_accepts_blocking", "src/check/types.rs", """if got != Type::MockResource {
                    return Err(mismatch(
                        Self::primary_span(ordered(0)),
                        Self::primary_span(ordered(0)),
                        None,
                        "MockResource",""", """if got != Type::MockResource && got != Type::MockBlockingResource {
                    return Err(mismatch(
                        Self::primary_span(ordered(0)),
                        Self::primary_span(ordered(0)),
                        None,
                        "MockResource",""", 0, "lib", "check::tests::resource_primitive_domains_match_manifest"),
    ("v55_lower_read_consume", "src/ir/lower.rs", 'vec![("r".into(), ParameterMode::Read, Ty::MockResource)]', 'vec![("r".into(), ParameterMode::Consume, Ty::MockResource)]', 0, "mock_oracle", "mock_oracle_runtime_contract"),
    ("v55_main_unreadable_reason", "src/main.rs", '            Err(_) => {\n                return driver_error(Some(&profile_id), "mock_env_unreadable", format == "json");\n            }', '            Err(_) => {\n                return driver_error(Some(&profile_id), "invalid_mock_json", format == "json");\n            }', 0, "mock_env_records_and_failures", "mock_environment_open_failures_report_unreadable_for_all_file_shapes"),
]
MUTATIONS += VAL55_MUTATIONS

M56_MUTATIONS = [
    ("m56_dup_during_parse", "src/interp/mock_env.rs", "            fields.push((key, value));", "            if fields.iter().any(|(k, _): &(String, Json)| *k == key) {\n                return Err(MockEnvironmentError::InvalidSchema);\n            }\n            fields.push((key, value));", 0, "i11a", "mock_json_syntax_error_precedes_duplicate_key_schema_error"),
    ("m56_trailing_one_byte_ok", "src/interp/mock_env.rs", "if self.at != self.bytes.len() {", "if self.at + 1 < self.bytes.len() {", 0, "mock_env_json_and_read_boundaries", "one_trailing_byte_after_a_complete_document_is_invalid_json"),
    ("m56_trailing_schema", "src/interp/mock_env.rs", "        if self.at != self.bytes.len() {\n            return Err(MockEnvironmentError::InvalidJson);", "        if self.at != self.bytes.len() {\n            return Err(MockEnvironmentError::InvalidSchema);", 0, "mock_env_json_and_read_boundaries", "one_trailing_byte_after_a_complete_document_is_invalid_json"),
    ("m56_depth_schema", "src/interp/mock_env.rs", "        if self.depth >= 16 {\n            return Err(MockEnvironmentError::InvalidJson);", "        if self.depth >= 16 {\n            return Err(MockEnvironmentError::InvalidSchema);", 0, "i11a", "mock_json_schema_and_canonical_i64_are_enforced"),
    ("m56_unterminated_string_schema", "src/interp/mock_env.rs", "        if self.peek() != Some(b'\"') {\n            return Err(MockEnvironmentError::InvalidJson);", "        if self.peek() != Some(b'\"') {\n            return Err(MockEnvironmentError::InvalidSchema);", 0, "mock_env_json_and_read_boundaries", "truncation_inside_a_json_string_is_invalid_json_not_schema"),
    ("m56_control_in_string_schema", "src/interp/mock_env.rs", "            if b < 0x20 {\n                return Err(MockEnvironmentError::InvalidJson);", "            if b < 0x20 {\n                return Err(MockEnvironmentError::InvalidSchema);", 0, "i11a", "mock_json_rejects_raw_0x1f_inside_string"),
    ("m56_canon_i64_saturate", "src/interp/mock_env.rs", "s.parse().map_err(|_| MockEnvironmentError::InvalidSchema)", "Ok(s.parse().unwrap_or(i64::MAX))", 0, "i11a", "mock_json_schema_and_canonical_i64_are_enforced"),
    ("m56_version_any_number", "src/interp/mock_env.rs", "Json::Number(ref n) if n == \"1\"", "Json::Number(_) ", 0, "i11a", "mock_json_uppercase_exponent_is_a_schema_error"),
    ("m56_result_any_key_err", "src/interp/mock_env.rs", "        \"err\" => canonical_i64(&fields[0].1).map(Err),\n        _ => Err(MockEnvironmentError::InvalidSchema),", "        \"err\" => canonical_i64(&fields[0].1).map(Err),\n        _ => canonical_i64(&fields[0].1).map(Err),", 0, "mock_env_json_and_read_boundaries", "result_objects_reject_every_key_other_than_ok_and_err"),
    ("m56_record_value_zero", "src/interp/mock_env.rs", "                    .map(|record| result(record, canonical_i64))", "                    .map(|record| result(record, |_| Ok(0)))", 0, "mock_oracle", "mock_oracle_compares_indexed_read_record_values_in_order"),
    ("m56_literal_true_trunc", "src/interp/mock_env.rs", "self.literal(b\"true\")?;", "self.literal(b\"tru\")?;", 0, "mock_env_records_and_failures", "mock_json_boolean_payloads_are_schema_errors"),
    ("m56_hex_decimal_only", "src/interp/mock_env.rs", ".and_then(|c| c.to_digit(16))", ".and_then(|c| c.to_digit(10))", 0, "mock_oracle", "mock_oracle_parser_contract"),
    ("m56_ws_formfeed", "src/interp/mock_env.rs", "matches!(b, b' ' | b'\\n' | b'\\r' | b'\\t')", "matches!(b, b' ' | b'\\n' | b'\\r' | b'\\t' | 0x0c)", 0, "mock_env_json_and_read_boundaries", "only_json_whitespace_bytes_separate_tokens"),
    ("m56_del_rejected", "src/interp/mock_env.rs", "            if b < 0x20 {", "            if b < 0x20 || b == 0x7f {", 0, "mock_env_json_and_read_boundaries", "raw_del_in_json_strings_is_valid_json_and_reaches_schema_validation"),
    ("m56_array_trailing_comma", "src/interp/mock_env.rs", "            self.expect(b',')?;\n        }\n        self.leave();\n        Ok(Json::Array(items))", "            self.expect(b',')?;\n            self.ws();\n            if self.take(b']') {\n                break;\n            }\n        }\n        self.leave();\n        Ok(Json::Array(items))", 0, "mock_env_json_and_read_boundaries", "trailing_commas_in_objects_and_arrays_are_invalid_json"),
    ("m56_object_trailing_comma", "src/interp/mock_env.rs", "            self.expect(b',')?;\n        }\n        self.leave();\n        Ok(Json::Object(fields))", "            self.expect(b',')?;\n            self.ws();\n            if self.take(b'}') {\n                break;\n            }\n        }\n        self.leave();\n        Ok(Json::Object(fields))", 0, "mock_env_json_and_read_boundaries", "trailing_commas_in_objects_and_arrays_are_invalid_json"),
    ("m56_surrogate_start", "src/interp/mock_env.rs", "(0xD800..=0xDBFF).contains(&first)", "(0xD801..=0xDBFF).contains(&first)", 0, "mock_oracle", "mock_oracle_parser_contract"),
    ("m56_exact5_dropped", "src/interp/mock_env.rs", "        if fields.len() != 5 {\n            return Err(MockEnvironmentError::InvalidSchema);\n        }\n        let version", "        let version", 0, "i11a", "mock_json_schema_and_canonical_i64_are_enforced"),
    ("m56_leading_zero_len2", "src/interp/mock_env.rs", "|| (s.starts_with('0') && s.len() > 1)", "|| (s.starts_with('0') && s.len() > 2)", 0, "mock_oracle", "mock_oracle_covers_non_string_literals_and_canonical_i64_at_every_slot"),
    ("m56_neg_zero_00", "src/interp/mock_env.rs", "s == \"-0\"", "s == \"-00\"", 0, "i11a", "mock_json_schema_and_canonical_i64_are_enforced"),
    ("m56_request_err_not_consumed", "src/interp/machine.rs", "            Intrinsic::MockRequest => {\n                self.mock_env.request.pop_front();", "            Intrinsic::MockRequest => {\n                if self.mock_env.request.front().is_some_and(|r| r.is_ok()) {\n                    self.mock_env.request.pop_front();\n                }", 0, "mock_oracle", "mock_oracle_all_primitives_cover_i64_extremes_and_both_queue_results"),
    ("m56_write_err_not_consumed", "src/interp/machine.rs", "            Intrinsic::MockWriteI64 => {\n                self.mock_env.write_i64.pop_front();", "            Intrinsic::MockWriteI64 => {\n                if self.mock_env.write_i64.front().is_some_and(|r| r.is_ok()) {\n                    self.mock_env.write_i64.pop_front();\n                }", 0, "mock_oracle", "mock_oracle_all_primitives_cover_i64_extremes_and_both_queue_results"),
    ("m56_read_empty_not_consumed", "src/interp/machine.rs", "            Intrinsic::MockReadRecords => {\n                self.mock_env.read_records.pop_front();", "            Intrinsic::MockReadRecords => {\n                if !matches!(self.mock_env.read_records.front(), Some(Ok(records)) if records.is_empty()) {\n                    self.mock_env.read_records.pop_front();\n                }", 0, "i11a", "mock_read_queue_consumes_one_reply_per_call"),
    ("m56_reply_err_consumes_two", "src/interp/machine.rs", "            Intrinsic::MockReply => {\n                self.mock_env.reply.pop_front();", "            Intrinsic::MockReply => {\n                if self.mock_env.reply.front().is_some_and(|r| r.is_err()) {\n                    self.mock_env.reply.pop_front();\n                }\n                self.mock_env.reply.pop_front();", 0, "mock_oracle", "mock_oracle_all_primitives_cover_i64_extremes_and_both_queue_results"),
    ("m56_read_event_ok_records_only", "src/interp/machine.rs", "Ok(records) => format!(\"read_records:{id}:ok:{}\", records.len()),", "Ok(records) => format!(\"read_records:{id}:ok:{}\", records.iter().filter(|record| record.is_ok()).count()),", 0, "mock_oracle", "mock_oracle_all_primitives_cover_i64_extremes_and_both_queue_results"),
    ("m56_read_cells_ok_records_only", "src/interp/machine.rs", "Ok(records) => 2u64.saturating_add((records.len() as u64).saturating_mul(2)),\n                    Err(_) => 2,\n                };\n                let work", "Ok(records) => 2u64.saturating_add((records.iter().filter(|record| record.is_ok()).count() as u64).saturating_mul(2)),\n                    Err(_) => 2,\n                };\n                let work", 0, "mock_env_json_and_read_boundaries", "read_err_and_mixed_record_reservations_cover_cells_64_boundaries"),
    ("m56_read_work_ok_records_only", "src/interp/machine.rs", "Ok(records) => 2u64.saturating_add((records.len() as u64).saturating_mul(2)),\n                    Err(_) => 2,\n                };\n                let event", "Ok(records) => 2u64.saturating_add((records.iter().filter(|record| record.is_ok()).count() as u64).saturating_mul(2)),\n                    Err(_) => 2,\n                };\n                let event", 0, "mock_oracle", "mock_oracle_runtime_contract"),
    ("m56_write_stdout_plus", "src/interp/machine.rs", "reply.as_ref().ok().map(|_| number.to_string());", "reply.as_ref().ok().map(|_| format!(\"{number:+}\"));", 0, "i11a", "write_ok_and_err_match_literal_json_contract_data"),
    ("m56_reply_err_swapped", "src/interp/machine.rs", "Err(error) => format!(\"reply:{status}:err:{error}\"),", "Err(error) => format!(\"reply:{error}:err:{status}\"),", 0, "mock_env_records_and_failures", "mock_reply_events_preserve_negative_status_sign_on_both_results"),
    ("m56_write_err_swapped", "src/interp/machine.rs", "Err(error) => format!(\"write_i64:{number}:err:{error}\"),", "Err(error) => format!(\"write_i64:{error}:err:{number}\"),", 0, "cli", "cli_json_preserves_err_types_channels_and_exit_status"),
    ("m56_read_event_resource_zero", "src/interp/machine.rs", "Ok(records) => format!(\"read_records:{id}:ok:{}\", records.len()),", "Ok(records) => format!(\"read_records:0:ok:{}\", records.len()),", 0, "mock_oracle", "mock_oracle_all_primitives_cover_i64_extremes_and_both_queue_results"),
    ("m56_request_exhausted_ok0", "src/interp/machine.rs", "let reply = self.mock_env.request.front().cloned().unwrap_or(Err(-9000));", "let reply = self.mock_env.request.front().cloned().unwrap_or(Ok(0));", 0, "mock_oracle", "mock_oracle_runtime_contract"),
    ("m56_reply_exhausted_ok", "src/interp/machine.rs", "let reply = self.mock_env.reply.front().cloned().unwrap_or(Err(-9000));", "let reply = self.mock_env.reply.front().cloned().unwrap_or(Ok(()));", 0, "mock_oracle", "mock_oracle_runtime_contract"),
    ("m56_write_work_inverted", "src/interp/machine.rs", "let work = 2 + u64::from(stdout.is_some());", "let work = 2 + u64::from(stdout.is_none());", 0, "mock_oracle", "mock_oracle_runtime_contract"),
    ("m56_unreadable_text_flag", "src/main.rs", "            Err(_) => {\n                return driver_error(Some(&profile_id), \"mock_env_unreadable\", format == \"json\");\n            }\n        };", "            Err(_) => {\n                return driver_error(Some(&profile_id), \"mock_env_unreadable\", false);\n            }\n        };", 0, "mock_env_json_and_read_boundaries", "unreadable_environment_is_exact_json_and_text_for_missing_and_directory"),
    ("m56_driver_error_exit1", "src/main.rs", "    ExitCode::from(2)\n}\n\nfn diagnostic_result", "    ExitCode::from(1)\n}\n\nfn diagnostic_result", 0, "mock_env_json_and_read_boundaries", "unreadable_environment_is_exact_json_and_text_for_missing_and_directory"),
    ("m56_too_large_reason_unreadable", "src/main.rs", "Err(InputError::TooLarge { .. }) => {\n                    return driver_error(Some(&profile_id), \"mock_env_too_large\", format == \"json\");", "Err(InputError::TooLarge { .. }) => {\n                    return driver_error(Some(&profile_id), \"mock_env_unreadable\", format == \"json\");", 0, "cli", "cli_reports_mock_environment_size_limit_reason"),
    ("m56_read_err_inner_json", "src/main.rs", "                Err(_) => {\n                    return driver_error(\n                        Some(&profile_id),\n                        \"mock_env_unreadable\",\n                        format == \"json\",\n                    );\n                }", "                Err(_) => {\n                    return driver_error(\n                        Some(&profile_id),\n                        \"invalid_mock_json\",\n                        format == \"json\",\n                    );\n                }", 0, "mock_env_json_and_read_boundaries", "read_failure_after_open_is_reported_as_unreadable"),
    ("m56_json_reason_as_schema", "src/main.rs", "            Err(nexo::interp::MockEnvironmentError::InvalidJson) => {\n                return driver_error(Some(&profile_id), \"invalid_mock_json\", format == \"json\");", "            Err(nexo::interp::MockEnvironmentError::InvalidJson) => {\n                return driver_error(Some(&profile_id), \"invalid_mock_schema\", format == \"json\");", 0, "mock_env_json_and_read_boundaries", "json_format_reports_invalid_json_reason_for_syntax_frontiers"),
    ("m56_check_write_untyped", "src/check/types.rs", "            \"MockOpen\" | \"MockBlockingOpen\" | \"MockWriteI64\" | \"MockReply\" => {\n                self.expr(ordered(0), Some(&Type::I64), None)?;", "            \"MockOpen\" | \"MockBlockingOpen\" | \"MockWriteI64\" | \"MockReply\" => {\n                self.expr(ordered(0), None, None)?;", 0, "lib", "check::tests::mock_write_i64_rejects_non_i64_argument"),
    ("m56_verify_request_type", "src/ir/verify.rs", "Intrinsic::MockRequest => Ty::Result(Box::new(Ty::I64), Box::new(Ty::I64)),\n                    _ => Ty::Unit,\n                })\n            }\n            Op::CallDirect {", "Intrinsic::MockRequest => Ty::Result(Box::new(Ty::I64), Box::new(Ty::Unit)),\n                    _ => Ty::Unit,\n                })\n            }\n            Op::CallDirect {", 0, "cli", "cli_mock_primitive_is_supported_after_i11a"),
]
MUTATIONS += M56_MUTATIONS

N57_MUTATIONS = [
    (
        "n57_frac_no_digit_ok",
        "src/interp/mock_env.rs",
        "if !self.peek().is_some_and(|b| b.is_ascii_digit()) {",
        "if self.peek().is_some_and(|b| b.is_ascii_digit()) {",
        0,
        "run_cli_diagnostics_and_cells",
        "malformed_and_noncanonical_json_numbers_keep_their_rejection_phase",
    ),
    (
        "n57_exp_no_digit_ok",
        "src/interp/mock_env.rs",
        "if !self.peek().is_some_and(|b| b.is_ascii_digit()) {",
        "if self.peek().is_some_and(|b| b.is_ascii_digit()) {",
        1,
        "run_cli_diagnostics_and_cells",
        "malformed_and_noncanonical_json_numbers_keep_their_rejection_phase",
    ),
    (
        "n57_exp_sign_plus_rejected",
        "src/interp/mock_env.rs",
        "matches!(b, b'+' | b'-')",
        "b == b'-'",
        0,
        "run_cli_diagnostics_and_cells",
        "malformed_and_noncanonical_json_numbers_keep_their_rejection_phase",
    ),
    (
        "n57_read_empty_cells1",
        "src/interp/machine.rs",
        "Ok(records) => 2u64.saturating_add((records.len() as u64).saturating_mul(2)),",
        "Ok(records) => 1u64.saturating_add((records.len() as u64).saturating_mul(2)),",
        0,
        "run_cli_diagnostics_and_cells",
        "cells64_read_reservations_cover_zero_one_two_and_large_ok_err_record_sets",
    ),
    (
        "n57_json_fault_exit_success",
        "src/main.rs",
        "nexo::interp::Outcome::Err(_) | nexo::interp::Outcome::SimulatorFault => {\n                ExitCode::FAILURE\n            }\n            nexo::interp::Outcome::InternalError(_) => ExitCode::from(2),",
        "nexo::interp::Outcome::Err(_) => {\n                ExitCode::FAILURE\n            }\n            nexo::interp::Outcome::InternalError(_) => ExitCode::from(2),",
        0,
        "run_cli_diagnostics_and_cells",
        "result_diagnostic_and_fault_classes_have_exact_outputs_in_both_formats",
    ),
    (
        "n57_text_fault_exit_success",
        "src/main.rs",
        "nexo::interp::Outcome::SimulatorFault => ExitCode::FAILURE,",
        "nexo::interp::Outcome::SimulatorFault => ExitCode::SUCCESS,",
        0,
        "run_cli_diagnostics_and_cells",
        "result_diagnostic_and_fault_classes_have_exact_outputs_in_both_formats",
    ),
    (
        "n57_format_repeat_ok",
        "src/main.rs",
        "if !seen_format\n                        && i + 1 < args.len()",
        "if (!seen_format || seen_format)\n                        && i + 1 < args.len()",
        0,
        "run_cli_diagnostics_and_cells",
        "run_option_surface_matrix_has_exact_usage_errors_and_accepts_supported_siblings",
    ),
    (
        "n57_profile_repeat_ok",
        "src/main.rs",
        "if !seen_profile\n                        && i + 1 < args.len()",
        "if (!seen_profile || seen_profile)\n                        && i + 1 < args.len()",
        0,
        "run_cli_diagnostics_and_cells",
        "run_option_surface_matrix_has_exact_usage_errors_and_accepts_supported_siblings",
    ),
    (
        "n57_mock_value_dashdash_ok",
        "src/main.rs",
        "&& !args[i + 1].to_string_lossy().starts_with(\"--\") =>",
        "=>",
        0,
        "run_cli_diagnostics_and_cells",
        "repeated_options_and_option_names_used_as_values_are_usage_errors",
    ),
    (
        "n57_invalid_format_accepted",
        "src/main.rs",
        "if format != \"text\" && format != \"json\" {\n                        return None;\n                    }",
        "if false {\n                        return None;\n                    }",
        0,
        "run_cli_diagnostics_and_cells",
        "run_option_surface_matrix_has_exact_usage_errors_and_accepts_supported_siblings",
    ),
    (
        "n57_unknown_option_ignored",
        "src/main.rs",
        "_ => return None,",
        "_ => { i += 1; },",
        0,
        "run_cli_diagnostics_and_cells",
        "run_option_surface_matrix_has_exact_usage_errors_and_accepts_supported_siblings",
    ),
]
MUTATIONS += N57_MUTATIONS

W58_MUTATIONS = [
    ("w58_next_resource_plus2", "src/interp/machine.rs", "self.next_resource += 1;", "self.next_resource += 2;", 0, "mock_oracle", "mock_oracle_all_primitives_cover_i64_extremes_and_both_queue_results"),
    ("w58_close_bytes_minus1", "src/interp/machine.rs", 'let close_bytes = ("mock-resource:".len() + close.len() + 1) as u64;', 'let close_bytes = ("mock-resource:".len() + close.len()) as u64;', 0, "run_cli_diagnostics_and_cells", "cells64_faults_exit_one_in_both_formats_and_empty_read_uses_two_cells"),
    ("w58_branch_ok_cells_keep", "src/interp/machine.rs", "                            self.cells = self.cells.saturating_sub(1);\n                            Some(*payload)", "                            Some(*payload)", 0, "run_cli_diagnostics_and_cells", "cells64_faults_exit_one_in_both_formats_and_empty_read_uses_two_cells"),
    ("w58_read_resource_not_kept_temp", "src/interp/machine.rs", "matches!(intrinsic, Intrinsic::Inspect | Intrinsic::MockReadRecords)\n                                && matches!(&value", "matches!(intrinsic, Intrinsic::Inspect)\n                                && matches!(&value", 0, "run_cli_diagnostics_and_cells", "cells64_faults_exit_one_in_both_formats_and_empty_read_uses_two_cells"),
    ("w58_dispose_cells_keep", "src/interp/machine.rs", "        self.cells = self.cells.saturating_sub(value.cells());\n        let mut active = value;", "        let mut active = value;", 0, "run_cli_diagnostics_and_cells", "cells64_faults_exit_one_in_both_formats_and_empty_read_uses_two_cells"),
    ("w58_open_work2", "src/interp/machine.rs", "                                if !self.charge_work(1, &ins.span) || !self.cell(&ins.span) {\n                                    break;\n                                }\n                                pre_reserved_output = true;\n                                let id = self.next_resource;", "                                if !self.charge_work(2, &ins.span) || !self.cell(&ins.span) {\n                                    break;\n                                }\n                                pre_reserved_output = true;\n                                let id = self.next_resource;", 0, "mock_oracle", "mock_oracle_runtime_contract"),
    ("w58_inspect_work2", "src/interp/machine.rs", 'let text = format!("inspect:{id}:{label}");\n                                if !self.charge_work(1, &ins.span)', 'let text = format!("inspect:{id}:{label}");\n                                if !self.charge_work(2, &ins.span)', 0, "lib", "ir::i10b_tests::published_work_table_totals_and_deltas_are_preserved"),
    ("w58_limit_cells_normal", "src/interp/machine.rs", "cells: 65_536,\n        observations: 65_536,", "cells: 65_535,\n        observations: 65_536,", 0, "lib", "interp::machine::tests::cell_boundary_rejects_proposed_occupation_without_committing_it"),
    ("w58_limit_obs_normal", "src/interp/machine.rs", "cells: 65_536,\n        observations: 65_536,", "cells: 65_536,\n        observations: 65_537,", 0, "lib", "ir::tests::observation_quota_checks_complete_next_line_and_preserves_prior_lines"),
    ("w58_limit_frames_normal", "src/interp/machine.rs", "frames: 256,", "frames: 257,", 0, "lib", "ir::tests::frame_limit_denies_acquisition_in_frame_257"),
    ("w58_limit_work_normal", "src/interp/machine.rs", "work: 1_000_000,", "work: 1_000_001,", 0, "i11a", "mock_primitive_work_costs_use_exact_normal_limit"),
    ("w58_limit_cells64", "src/interp/machine.rs", "cells: 64,\n            ..ExecutionLimits::NORMAL", "cells: 65,\n            ..ExecutionLimits::NORMAL", 0, "run_cli_diagnostics_and_cells", "cells64_faults_exit_one_in_both_formats_and_empty_read_uses_two_cells"),
    ("w58_limit_obs64", "src/interp/machine.rs", "observations: 64,\n            ..ExecutionLimits::NORMAL", "observations: 65,\n            ..ExecutionLimits::NORMAL", 0, "runtime_profiles_and_resource_limits", "reduced_observation_profile_checks_all_sides_of_the_limit_with_and_without_a_resource"),
    ("w58_limit_frames8", "src/interp/machine.rs", "frames: 8,\n            ..ExecutionLimits::NORMAL", "frames: 9,\n            ..ExecutionLimits::NORMAL", 0, "runtime_profiles_and_resource_limits", "reduced_frames_profile_accepts_seven_and_eight_and_rejects_nine_frames"),
    ("w58_profile_effects32_id", "src/interp/machine.rs", 'profile_id: "f0-0002-b-test-effects-32-v2",', 'profile_id: "f0-0002-b-test-effects-32-v1",', 0, "runtime_profiles_and_resource_limits", "selected_profile_is_preserved_in_runtime_and_static_diagnostics"),
    ("w58_log_reserve_ge", "src/interp/observation.rs", "        if proposed > self.limit {\n            return Err(proposed);\n        }\n        self.reserved += bytes;", "        if proposed >= self.limit {\n            return Err(proposed);\n        }\n        self.reserved += bytes;", 0, "runtime_profiles_and_resource_limits", "observations_reservation_uses_complete_proposal_and_preserves_prior_lines"),
    ("w58_log_emit_reserved_no_release", "src/interp/observation.rs", "        self.reserved -= n;\n        self.bytes = self.bytes.checked_add(n)", "        self.bytes = self.bytes.checked_add(n)", 0, "run_cli_diagnostics_and_cells", "cells64_read_reservations_cover_zero_one_two_and_large_ok_err_record_sets"),
    ("w58_effects_write_pure", "src/ir/effects.rs", "Print | Inspect | MockOpen | MockBlockingOpen | MockWriteI64 | MockRequest | MockReply => {", 'MockWriteI64 => (0, 0, "intrinsic.call"),\n        Print | Inspect | MockOpen | MockBlockingOpen | MockRequest | MockReply => {', 0, "cli", "cli_mock_write_requires_io_effect"),
    ("w58_effects_reply_pure", "src/ir/effects.rs", "Print | Inspect | MockOpen | MockBlockingOpen | MockWriteI64 | MockRequest | MockReply => {", 'MockReply => (0, 0, "intrinsic.call"),\n        Print | Inspect | MockOpen | MockBlockingOpen | MockWriteI64 | MockRequest => {', 0, "cli", "cli_request_requires_io_effect_and_reply_effect_name_is_specific"),
    ("w58_effects_read_no_io", "src/ir/effects.rs", "MockReadRecords => (ALLOC | IO, 0, \"intrinsic.call\"),", "MockReadRecords => (ALLOC, 0, \"intrinsic.call\"),", 0, "runtime_profiles_and_resource_limits", "mock_intrinsics_declare_io_and_read_records_also_declares_alloc"),
    ("w58_effects_open_pure", "src/ir/effects.rs", "Print | Inspect | MockOpen | MockBlockingOpen | MockWriteI64 | MockRequest | MockReply => {", 'MockOpen => (0, 0, "intrinsic.call"),\n        Print | Inspect | MockBlockingOpen | MockWriteI64 | MockRequest | MockReply => {', 0, "lib", "ir::effects::witness_differential_tests::witness_prefers_total_cause_count_before_callsite_offset"),
    ("w58_names_reply_label", "src/check/names.rs", 'vec![("status", I64)],', 'vec![("code", I64)],', 0, "lib", "check::tests::manifest_primitives_accept_only_their_written_labels_and_results"),
    ("w58_names_write_label", "src/check/names.rs", '"MockWriteI64" => (\n            vec![("n", I64)],', '"MockWriteI64" => (\n            vec![("value", I64)],', 0, "lib", "check::tests::manifest_primitives_accept_only_their_written_labels_and_results"),
    ("w58_names_read_label", "src/check/names.rs", '"MockReadRecords" => (\n            vec![("r", MockResource)],', '"MockReadRecords" => (\n            vec![("resource", MockResource)],', 0, "lib", "check::tests::manifest_primitives_accept_only_their_written_labels_and_results"),
    ("w58_main_rejected_exit2", "src/main.rs", '"rejected",\n                    &errors,\n                    1,', '"rejected",\n                    &errors,\n                    2,', 0, "run_cli_diagnostics_and_cells", "mock_environment_and_compilation_diagnostics_match_exact_contract_outputs"),
    ("w58_main_unreadable_exit1", "src/main.rs", "                d,\n                2,\n                SourceView {\n                    bytes_len: 0,", "                d,\n                1,\n                SourceView {\n                    bytes_len: 0,", 0, "cli", "cli_reports_unreadable_and_invalid_utf8_before_parsing"),
    ("w58_main_toolarge_exit2", "src/main.rs", "                d,\n                1,\n                SourceView {\n                    bytes_len: observed,", "                d,\n                2,\n                SourceView {\n                    bytes_len: observed,", 0, "cli", "cli_source_byte_boundary_precedes_utf8_decoding"),
    ("w58_main_diag_profile_default", "src/main.rs", "                error.profile_id = profile_id.clone();\n", "                let _ = &profile_id;\n", 0, "runtime_profiles_and_resource_limits", "compile_time_diagnostics_and_unknown_profile_json_keep_selected_profile_ids"),
    ("w58_main_unknown_profile_text", "src/main.rs", '"unknown_profile", format == "json")', '"unknown_profile", false)', 0, "runtime_profiles_and_resource_limits", "profile_selection_matrix_covers_defaults_all_profiles_and_unknown_ids"),
    ("w58_main_json_bytes_swap", "src/main.rs", "                execution.observations.bytes(),\n                execution.observations.reserved_bytes(),", "                execution.observations.reserved_bytes(),\n                execution.observations.bytes(),", 0, "cli", "cli_json_preserves_err_types_channels_and_exit_status"),
    ("w58_main_diag_before_obs", "src/main.rs", "    for observation in execution.observations.observations() {\n        match observation.channel {\n            nexo::interp::observation::Channel::Stdout => println!(\"{}\", observation.text),\n            nexo::interp::observation::Channel::Event => {\n                eprintln!(\"{}{}\", observation.prefix, observation.text)\n            }\n        }\n    }\n", "", 0, "run_cli_diagnostics_and_cells", "result_diagnostic_and_fault_classes_have_exact_outputs_in_both_formats"),
    ("w58_main_explain_err_exit1", "src/main.rs", "        eprintln!(\"nexo-explain:{reason}\");\n    }\n    ExitCode::from(2)", "        eprintln!(\"nexo-explain:{reason}\");\n    }\n    ExitCode::from(1)", 0, "i01_diagnostic", "explain_cli_reports_unknown_code_subcause_and_argument_errors"),
    ("w58_main_explain_ok_exit1", "src/main.rs", "                print!(\"{}\", nexo::diagnostic::explanation_text(&entries));\n            }\n            ExitCode::SUCCESS", "                print!(\"{}\", nexo::diagnostic::explanation_text(&entries));\n            }\n            ExitCode::FAILURE", 0, "i01_diagnostic", "explain_json_is_single_line_and_has_documented_error_shape"),
    ("w58_main_check_json_exit", "src/main.rs", 'execution_json(&profile_id, r#"{"kind":"checked"}"#, &[], 0, 0, &[])\n            );\n            return ExitCode::SUCCESS;', 'execution_json(&profile_id, r#"{"kind":"checked"}"#, &[], 0, 0, &[])\n            );\n            return ExitCode::from(1);', 0, "run_cli_diagnostics_and_cells", "run_option_surface_matrix_has_exact_usage_errors_and_accepts_supported_siblings"),
    ("w58_main_ok_text_stdout", "src/main.rs", 'eprintln!("nexo-result:Ok(())");', 'println!("nexo-result:Ok(())");', 0, "cli", "cli_result_outcomes_keep_err_separate_from_runtime_faults"),
    ("w58_main_err_json_kind", "src/main.rs", 'r#"{{"kind":"err","error":"{error}"}}"#', 'r#"{{"kind":"err","error":{error}}}"#', 0, "cli", "cli_json_preserves_err_types_channels_and_exit_status"),
    ("w58_interp_readindex_work1", "src/interp/machine.rs", "                        if !self.charge_work(selected.cells(), &ins.span)\n                            || !self.reserve_cells(selected.cells(), &ins.span)\n                        {\n                            break;\n                        }\n                        Some(selected.deep_copy())\n                    }\n                    Op::CheckIndex {", "                        if !self.charge_work(1, &ins.span)\n                            || !self.reserve_cells(selected.cells(), &ins.span)\n                        {\n                            break;\n                        }\n                        Some(selected.deep_copy())\n                    }\n                    Op::CheckIndex {", 0, "runtime_profiles_and_resource_limits", "indexed_result_element_copy_uses_each_cell_at_the_work_frontier"),
    ("w58_initial_entry_skips_resource_cleanup", "src/interp/machine.rs", "                        if !self.charge_work(1, &ins.span) {\n                            self.dispose_intrinsic_resources(&mut values)?;\n                            break;\n                        }", "                        if !self.charge_work(1, &ins.span) {\n                            break;\n                        }", 0, "runtime_profiles_and_resource_limits", "entry_tick_exhaustion_closes_every_temporary_resource_primitive"),
    ("w58_drop_entry_skips_resource_cleanup", "src/interp/machine.rs", "                            if !self.charge_work(1, &ins.span) {\n                                self.dispose_intrinsic_resources(&mut values)?;\n                                break;\n                            }", "                            if !self.charge_work(1, &ins.span) {\n                                break;\n                            }", 0, "runtime_profiles_and_resource_limits", "entry_tick_exhaustion_closes_every_temporary_resource_primitive"),
    ("w58_drop_reserve_before_dispose", "src/interp/machine.rs", "                        for (id, value) in argument_ids.into_iter().zip(values).rev() {", "                        if *intrinsic == Intrinsic::DropData && !self.cell(&ins.span) {\n                            break;\n                        }\n                        for (id, value) in argument_ids.into_iter().zip(values).rev() {", 0, "runtime_profiles_and_resource_limits", "drop_destroys_resource_before_materializing_unit_at_the_cell_boundary"),
    ("w58_catalog_profile_old_id", "docs/adr/F0-CODIGOS.md", "`f0-0002-b-test-effects-32-v2`", "`f0-0002-test-effects-32-v2`", 0, "runtime_profiles_and_resource_limits", "profile_catalog_matches_official_b_cli_identifiers"),
    ("w58_normal_work_minus1", "src/interp/machine.rs", "work: 1_000_000,", "work: 999_999,", 0, "runtime_profiles_and_resource_limits", "runtime_profile_table_drives_work_frames_cells_and_observation_boundaries"),
    ("w58_normal_frames_minus1", "src/interp/machine.rs", "frames: 256,", "frames: 255,", 0, "runtime_profiles_and_resource_limits", "runtime_profile_table_drives_work_frames_cells_and_observation_boundaries"),
    ("w58_normal_cells_plus1", "src/interp/machine.rs", "cells: 65_536,\n        observations: 65_536,", "cells: 65_537,\n        observations: 65_536,", 0, "runtime_profiles_and_resource_limits", "runtime_profile_table_drives_work_frames_cells_and_observation_boundaries"),
    ("w58_normal_obs_minus1", "src/interp/machine.rs", "cells: 65_536,\n        observations: 65_536,", "cells: 65_536,\n        observations: 65_535,", 0, "runtime_profiles_and_resource_limits", "runtime_profile_table_drives_work_frames_cells_and_observation_boundaries"),
    ("w58_frames8_minus1", "src/interp/machine.rs", "frames: 8,\n            ..ExecutionLimits::NORMAL", "frames: 7,\n            ..ExecutionLimits::NORMAL", 0, "runtime_profiles_and_resource_limits", "reduced_frames_profile_accepts_seven_and_eight_and_rejects_nine_frames"),
    ("w58_cells64_minus1", "src/interp/machine.rs", "cells: 64,\n            ..ExecutionLimits::NORMAL", "cells: 63,\n            ..ExecutionLimits::NORMAL", 0, "runtime_profiles_and_resource_limits", "runtime_profile_table_drives_work_frames_cells_and_observation_boundaries"),
    ("w58_obs64_minus1", "src/interp/machine.rs", "observations: 64,\n            ..ExecutionLimits::NORMAL", "observations: 63,\n            ..ExecutionLimits::NORMAL", 0, "runtime_profiles_and_resource_limits", "reduced_observation_profile_checks_all_sides_of_the_limit_with_and_without_a_resource"),
    ("w58_types32_minus1", "src/ir/lower.rs", "type_solver_steps: 32,", "type_solver_steps: 31,", 0, "runtime_profiles_and_resource_limits", "reduced_types32_profile_reports_the_exact_source_budget_boundary"),
    ("w58_flow32_plus1", "src/ir/lower.rs", "flow_function: 32,", "flow_function: 33,", 0, "lib", "ir::i10a_tests::reduced_analysis_profiles_are_selected_through_internal_rust_api"),
    ("w58_flow32_minus1", "src/ir/lower.rs", "flow_function: 32,", "flow_function: 31,", 0, "lib", "ir::i10a_tests::reduced_analysis_profiles_are_selected_through_internal_rust_api"),
    ("w58_flow_global32_plus1", "src/ir/lower.rs", "flow_compilation: 32,", "flow_compilation: 33,", 0, "lib", "ir::i10a_tests::reduced_analysis_profiles_are_selected_through_internal_rust_api"),
    ("w58_flow_global32_minus1", "src/ir/lower.rs", "flow_compilation: 32,", "flow_compilation: 31,", 0, "lib", "ir::i10a_tests::reduced_analysis_profiles_are_selected_through_internal_rust_api"),
    ("w58_effects32_plus1", "src/ir/lower.rs", "effects: 32,", "effects: 33,", 0, "lib", "ir::i10a_tests::effect_program_accepts_exact_budget_and_rejects_first_excess"),
    ("w58_effects32_minus1", "src/ir/lower.rs", "effects: 32,", "effects: 31,", 0, "lib", "ir::i10a_tests::effect_program_accepts_exact_budget_and_rejects_first_excess"),
    ("w58_ir32_plus1", "src/ir/lower.rs", "ir_items: 32,", "ir_items: 33,", 0, "lib", "ir::i10a_tests::reduced_analysis_profiles_are_selected_through_internal_rust_api"),
    ("w58_ir32_minus1", "src/ir/lower.rs", "ir_items: 32,", "ir_items: 31,", 0, "lib", "ir::i10a_tests::reduced_analysis_profiles_are_selected_through_internal_rust_api"),
    ("w58_normal_types_minus1", "src/ir/lower.rs", "type_solver_steps: 1_000_000,", "type_solver_steps: 999_999,", 0, "runtime_profiles_and_resource_limits", "normal_type_solver_limit_is_reached_by_repeated_deep_array_comparisons"),
    ("w58_normal_types_plus1", "src/ir/lower.rs", "type_solver_steps: 1_000_000,", "type_solver_steps: 1_000_001,", 0, "runtime_profiles_and_resource_limits", "normal_type_solver_limit_is_reached_by_repeated_deep_array_comparisons"),
    ("w58_normal_flow_function_minus1", "src/ir/lower.rs", "flow_function: 1_000_000,", "flow_function: 999_999,", 0, "lib", "ir::i10a_tests::ownership_flow_limits_distinguish_local_and_compilation_scope"),
    ("w58_normal_flow_function_plus1", "src/ir/lower.rs", "flow_function: 1_000_000,", "flow_function: 1_000_001,", 0, "lib", "ir::i10a_tests::ownership_flow_limits_distinguish_local_and_compilation_scope"),
    ("w58_normal_flow_global_minus1", "src/ir/lower.rs", "flow_compilation: 4_000_000,", "flow_compilation: 3_999_999,", 0, "lib", "ir::i10a_tests::ownership_flow_limits_distinguish_local_and_compilation_scope"),
    ("w58_normal_flow_global_plus1", "src/ir/lower.rs", "flow_compilation: 4_000_000,", "flow_compilation: 4_000_001,", 0, "lib", "ir::i10a_tests::ownership_flow_limits_distinguish_local_and_compilation_scope"),
    ("w58_normal_effects_minus1", "src/ir/lower.rs", "effects: 1_000_000,", "effects: 999_999,", 0, "lib", "ir::i10a_tests::effect_fixpoint_charges_each_operation_and_callsite_to_global_limit"),
    ("w58_normal_effects_plus1", "src/ir/lower.rs", "effects: 1_000_000,", "effects: 1_000_001,", 0, "lib", "ir::i10a_tests::effect_fixpoint_charges_each_operation_and_callsite_to_global_limit"),
    ("w58_normal_ir_items_minus1", "src/ir/lower.rs", "ir_items: 262_144,", "ir_items: 262_143,", 0, "lib", "ir::i10a_tests::analysis_profile_limit_table_matches_the_published_values"),
    ("w58_normal_ir_items_plus1", "src/ir/lower.rs", "ir_items: 262_144,", "ir_items: 262_145,", 0, "lib", "ir::i10a_tests::analysis_profile_limit_table_matches_the_published_values"),
    ("w58_r_profile_defaults", "src/interp/machine.rs", "profile_id: self.limits.profile_id.into(),", 'profile_id: diagnostic::PROFILE_ID.into(),', 0, "runtime_profiles_and_resource_limits", "runtime_profile_table_drives_work_frames_cells_and_observation_boundaries"),
    ("w58_r_work_limit_plus1", "src/interp/machine.rs", '("limit", DataValue::UInt(self.limits.work)),', '("limit", DataValue::UInt(self.limits.work.saturating_add(1))),', 0, "runtime_profiles_and_resource_limits", "runtime_profile_table_drives_work_frames_cells_and_observation_boundaries"),
    ("w58_r_work_observed_plus1", "src/interp/machine.rs", 'DataValue::UInt(self.limits.work.saturating_add(1)),', 'DataValue::UInt(self.limits.work.saturating_add(2)),', 0, "runtime_profiles_and_resource_limits", "runtime_profile_table_drives_work_frames_cells_and_observation_boundaries"),
    ("w58_r_work_consumed_plus1", "src/interp/machine.rs", '("work_consumed", DataValue::UInt(self.limits.work)),', '("work_consumed", DataValue::UInt(self.limits.work.saturating_add(1))),', 0, "runtime_profiles_and_resource_limits", "runtime_profile_table_drives_work_frames_cells_and_observation_boundaries"),
    ("w58_r_frames_observed_plus1", "src/interp/machine.rs", 'DataValue::UInt(self.limits.frames.saturating_add(1)),', 'DataValue::UInt(self.limits.frames.saturating_add(2)),', 0, "runtime_profiles_and_resource_limits", "runtime_profile_table_drives_work_frames_cells_and_observation_boundaries"),
    ("w58_r_cells_observed_plus1", "src/interp/machine.rs", '("observed", DataValue::UInt(proposed)),', '("observed", DataValue::UInt(proposed.saturating_add(1))),', 0, "runtime_profiles_and_resource_limits", "runtime_profile_table_drives_work_frames_cells_and_observation_boundaries"),
    ("w58_r_observation_observed_plus1", "src/interp/machine.rs", '("observed", DataValue::UInt(observed)),', '("observed", DataValue::UInt(observed.saturating_add(1))),', 0, "runtime_profiles_and_resource_limits", "runtime_profile_table_drives_work_frames_cells_and_observation_boundaries"),
    ("w58_main_profile_default", "src/main.rs", "let mut profile = diagnostic::PROFILE_ID.to_owned();", 'let mut profile = "bogus".to_owned();', 0, "runtime_profiles_and_resource_limits", "profile_selection_matrix_covers_defaults_all_profiles_and_unknown_ids"),
    ("w58_main_format_default", "src/main.rs", 'let mut format = "text";', 'let mut format = "json";', 0, "runtime_profiles_and_resource_limits", "profile_selection_matrix_covers_defaults_all_profiles_and_unknown_ids"),
    ("w58_main_profile_value_ignored", "src/main.rs", "profile = args[i].to_str()?.to_owned();", "profile = diagnostic::PROFILE_ID.to_owned();", 0, "runtime_profiles_and_resource_limits", "profile_selection_matrix_covers_defaults_all_profiles_and_unknown_ids"),
    ("w58_main_format_value_ignored", "src/main.rs", "format = args[i].to_str()?;", 'format = "text";', 0, "runtime_profiles_and_resource_limits", "profile_selection_matrix_covers_defaults_all_profiles_and_unknown_ids"),
]
MUTATIONS += W58_MUTATIONS

VAL59_MUTATIONS = [
    ("x59_op_r0002_not_runtime_limit", "src/interp/machine.rs", '"R0001" | "R0002" => Operation::RuntimeLimit,', '"R0001" => Operation::RuntimeLimit,', 0, "run_cli_diagnostics_and_cells", "cells64_faults_exit_one_in_both_formats_and_empty_read_uses_two_cells"),
    ("x59_cells_kind_name", "src/interp/machine.rs", 'DataValue::String("live_value_cells".into()),\n                    ),\n                    ("limit", DataValue::UInt(self.limits.cells)),', 'DataValue::String("live_cells".into()),\n                    ),\n                    ("limit", DataValue::UInt(self.limits.cells)),', 0, "run_cli_diagnostics_and_cells", "cells64_faults_exit_one_in_both_formats_and_empty_read_uses_two_cells"),
    ("x59_print_array_sep", "src/interp/machine.rs", '.join(", ")', '.join(",")', 0, "runtime_profiles_and_resource_limits", "runtime_profile_table_drives_work_frames_cells_and_observation_boundaries"),
    ("x59_print_result_tag", "src/interp/machine.rs", 'if *ok { "Ok" } else { "Err" }', 'if *ok { "Err" } else { "Ok" }', 0, "mock_oracle", "mock_oracle_compares_indexed_read_record_values_in_order"),
    ("x59_try_add_err31", "src/interp/machine.rs", "payload: Box::new(Value::Int(30)),", "payload: Box::new(Value::Int(31)),", 0, "cli", "cli_result_outcomes_keep_err_separate_from_runtime_faults"),
    ("x59_frames_gt", "src/interp/machine.rs", "if self.frames >= self.limits.frames {", "if self.frames > self.limits.frames {", 0, "runtime_profiles_and_resource_limits", "reduced_frames_profile_accepts_seven_and_eight_and_rejects_nine_frames"),
    ("x59_equal_arr_hdr1", "src/interp/machine.rs", "let mut work = 2u64;\n            if a.len() != b.len() {", "let mut work = 1u64;\n            if a.len() != b.len() {", 0, "lib", "ir::i10b_tests::equality_stops_at_the_first_different_payload_pair"),
    ("x59_equal_result_tag4", "src/interp/machine.rs", "if a != b {\n                2\n            } else {", "if a != b {\n                4\n            } else {", 0, "lib", "ir::i10b_tests::drop_work_and_result_tag_equality_stop_at_the_first_pair"),
    ("x59_json_err_exit0", "src/main.rs", "nexo::interp::Outcome::Err(_) | nexo::interp::Outcome::SimulatorFault => {\n                ExitCode::FAILURE\n            }", "nexo::interp::Outcome::SimulatorFault => {\n                ExitCode::FAILURE\n            }", 0, "cli", "cli_json_preserves_err_types_channels_and_exit_status"),
    ("x59_text_err_exit0", "src/main.rs", 'eprintln!("nexo-result:Err({error})");\n            ExitCode::FAILURE', 'eprintln!("nexo-result:Err({error})");\n            ExitCode::SUCCESS', 0, "cli", "cli_result_outcomes_keep_err_separate_from_runtime_faults"),
    ("x59_explain_second_positional", "src/main.rs", 'value if !value.starts_with(\'-\') && code.is_none() => {', 'value if !value.starts_with(\'-\') => {', 0, "i01_diagnostic", "explain_cli_reports_unknown_code_subcause_and_argument_errors"),
    ("x59_explain_subcause_repeat", "src/main.rs", '"--subcause" if subcause.is_none() && i + 1 < args.len() => {', '"--subcause" if i + 1 < args.len() => {', 0, "explain_and_print_output", "explain_rejects_a_repeated_subcause_even_when_every_value_is_valid"),
    ("x59_explain_format_any", "src/main.rs", '"--format"\n                if !format_seen\n                    && i + 1 < args.len()\n                    && matches!(args[i + 1].as_str(), "text" | "json") =>', '"--format" if !format_seen && i + 1 < args.len() =>', 0, "explain_and_print_output", "explain_format_accepts_only_text_or_json"),
    ("x59_obs_push_ge", "src/interp/observation.rs", "if proposed > self.limit {\n            return Err(proposed);\n        }\n        self.bytes += n;", "if proposed >= self.limit {\n            return Err(proposed);\n        }\n        self.bytes += n;", 0, "runtime_profiles_and_resource_limits", "reduced_observation_profile_checks_all_sides_of_the_limit_with_and_without_a_resource"),
    ("x59_close_text_swap", "src/interp/machine.rs", '.emit_reserved_event(format!("close:{id}:{label}"), "mock-resource:")?;', '.emit_reserved_event(format!("close:{label}:{id}"), "mock-resource:")?;', 0, "mock_oracle", "mock_oracle_all_primitives_cover_i64_extremes_and_both_queue_results"),
    ("x59_inspect_text_swap", "src/interp/machine.rs", 'let text = format!("inspect:{id}:{label}");', 'let text = format!("inspect:{label}:{id}");', 0, "cli", "cli_preserves_corpus_semantics_and_events"),
    ("x59_read_count_plus1", "src/interp/machine.rs", 'Ok(records) => format!("read_records:{id}:ok:{}", records.len()),', 'Ok(records) => format!("read_records:{id}:ok:{}", records.len() + 1),', 0, "run_cli_diagnostics_and_cells", "cells64_read_reservations_cover_zero_one_two_and_large_ok_err_record_sets"),
    ("x59_reply_err_text", "src/interp/machine.rs", 'Err(error) => format!("reply:{status}:err:{error}"),', 'Err(error) => format!("reply:{status}:{error}"),', 0, "mock_oracle", "mock_oracle_all_primitives_cover_i64_extremes_and_both_queue_results"),
    ("x59_cells_observed_minus1", "src/interp/machine.rs", '("limit", DataValue::UInt(self.limits.cells)),\n                    ("observed", DataValue::UInt(proposed)),', '("limit", DataValue::UInt(self.limits.cells)),\n                    ("observed", DataValue::UInt(proposed.saturating_sub(1))),', 0, "run_cli_diagnostics_and_cells", "cells64_faults_exit_one_in_both_formats_and_empty_read_uses_two_cells"),
    ("x59_cells_limit_plus1", "src/interp/machine.rs", '("limit", DataValue::UInt(self.limits.cells)),\n                    ("observed", DataValue::UInt(proposed)),', '("limit", DataValue::UInt(self.limits.cells + 1)),\n                    ("observed", DataValue::UInt(proposed)),', 0, "run_cli_diagnostics_and_cells", "cells64_faults_exit_one_in_both_formats_and_empty_read_uses_two_cells"),
    ("x59_frames_limit_field", "src/interp/machine.rs", '("limit", DataValue::UInt(self.limits.frames)),', '("limit", DataValue::UInt(self.limits.frames + 1)),', 0, "runtime_profiles_and_resource_limits", "reduced_frames_profile_accepts_seven_and_eight_and_rejects_nine_frames"),
    ("x59_explain_text_newline", "src/main.rs", 'print!("{}", nexo::diagnostic::explanation_text(&entries));', 'println!("{}", nexo::diagnostic::explanation_text(&entries));', 0, "explain_and_print_output", "explain_text_separates_entries_and_ends_with_one_lf"),
    ("x59_check_ok_text", "src/main.rs", 'println!("nexo-check:ok");', 'println!("nexo-check:OK");', 0, "cli", "cli_check_and_run_use_official_b_pipeline"),
    ("x59_driver_text_space", "src/main.rs", 'eprintln!("nexo-driver:{reason}");', 'eprintln!("nexo-driver: {reason}");', 0, "cli", "cli_rejects_options_before_reading_any_source"),
    ("x59_r0102_operation", "src/interp/machine.rs", '"R0102" => Operation::ValueIndex,', '"R0102" => Operation::ValueAdd,', 0, "lib", "interp::machine::tests::indexed_edit_loan_checks_each_index_before_entering_callee"),
    ("x59_doc_cells64_codigos", "docs/adr/F0-CODIGOS.md", '| `f0-0002-b-test-cells-64-v2` | live_value_cells = 64. |', '| `f0-0002-b-test-cells-64-v2` | live_value_cells = 65. |', 0, "explain_and_print_output", "published_reduced_profile_quotas_match_the_catalog_and_d15"),
    ("x59_doc_cells64_d15", "docs/adr/F0-D15-sintaxis-oficial.md", '| `f0-0002-b-test-cells-64-v2` | `live_value_cells = 64` |', '| `f0-0002-b-test-cells-64-v2` | `live_value_cells = 65` |', 0, "explain_and_print_output", "published_reduced_profile_quotas_match_the_catalog_and_d15"),
    ("y59_retained_no_rev", "src/interp/machine.rs", "for value in retained.into_iter().rev() {", "for value in retained.into_iter() {", 0, "lib", "ir::i05_tests::borrowed_owner_temporaries_close_in_reverse_written_order"),
    ("y59_temps_no_rev", "src/interp/machine.rs", "for v in temps.into_iter().rev().flatten() {", "for v in temps.into_iter().flatten() {", 0, "lib", "ir::i05_tests::failed_argument_cleans_prepared_temporaries_before_locals_in_reverse_order"),
    ("y59_text_bool_digit", "src/interp/machine.rs", "Self::Bool(b) => b.to_string(),", 'Self::Bool(b) => if *b { "1".into() } else { "0".into() },', 0, "cli", "cli_array_equality_releases_loans_before_write"),
    ("y59_text_unit_word", "src/interp/machine.rs", 'Self::Unit => "()".into(),', 'Self::Unit => "unit".into(),', 0, "explain_and_print_output", "print_formats_unit_arrays_and_result_values_exactly"),
    ("y59_len_plus1", "src/interp/machine.rs", 'i64::try_from(xs.len()).map_err(|_| "array length overflow")?;', 'i64::try_from(xs.len() + 1).map_err(|_| "array length overflow")?;', 0, "mock_oracle", "mock_oracle_runtime_contract"),
    ("y59_batch_ge", "src/interp/observation.rs", "if proposed > self.limit {\n            return Err(proposed);\n        }\n        self.bytes += added;", "if proposed >= self.limit {\n            return Err(proposed);\n        }\n        self.bytes += added;", 0, "i11a", "observation_limits_are_atomic_and_include_close_reservations"),
    ("y59_explain_err_code_null", "src/diagnostic.rs", 'code.map(q).unwrap_or_else(|| "null".into()),', '"null".to_owned(),', 0, "i01_diagnostic", "explain_json_is_single_line_and_has_documented_error_shape"),
    ("y59_explain_err_subcause_null", "src/diagnostic.rs", 'subcause.map(q).unwrap_or_else(|| "null".into())\n    )', '"null".to_owned()\n    )', 0, "i01_diagnostic", "explain_json_is_single_line_and_has_documented_error_shape"),
    ("y59_explain_json_sep", "src/diagnostic.rs", "if i > 0 {\n            out.push(',');\n        }\n        write!(out,", "if i > 0 {\n            out.push(';');\n        }\n        write!(out,", 0, "explain_and_print_output", "explain_json_entries_are_comma_separated_in_a_single_object"),
    ("y59_explain_sort_rev", "src/diagnostic.rs", "causes.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));", "causes.sort_by(|a, b| b.as_bytes().cmp(a.as_bytes()));", 0, "i01_diagnostic", "explain_subcauses_are_returned_in_lexicographic_order"),
    ("y59_json_err_unquoted", "src/main.rs", 'format!(r#"{{"kind":"err","error":"{error}"}}"#)', 'format!(r#"{{"kind":"err","error":{error}}}"#)', 0, "cli", "cli_json_preserves_err_types_channels_and_exit_status"),
    ("y59_json_ok_kind", "src/main.rs", 'r#"{"kind":"ok"}"#.to_owned()', 'r#"{"kind":"Ok"}"#.to_owned()', 0, "cli", "cli_json_preserves_err_types_channels_and_exit_status"),
    ("y59_text_ok_result", "src/main.rs", 'eprintln!("nexo-result:Ok(())");', 'eprintln!("nexo-result:Ok");', 0, "cli", "cli_result_outcomes_keep_err_separate_from_runtime_faults"),
    ("y59_explain_nx0006_hist", "src/diagnostic.rs", 'if *id == "implementation_limit" {\n                    "historical"', 'if *id == "implementation_limit_" {\n                    "historical"', 0, "lib", "diagnostic::tests::b_feature_catalog_excludes_legacy_and_validates_limit_scope"),
    ("y59_explain_nx0006_subcause", "src/diagnostic.rs", 'if code == "NX-E0006" {\n        if subcause.is_some() {\n            return Err(ExplainError::UnknownSubcause);', 'if code == "NX-E0006" {\n        if subcause.is_some() {\n            return Err(ExplainError::UnknownCode);', 0, "i01_diagnostic", "explain_cli_reports_unknown_code_subcause_and_argument_errors"),
    ("v59_open_acquisition_observed", "src/interp/machine.rs", "let acquisition_bytes = close_bytes + open_bytes;", "let acquisition_bytes = close_bytes;", 0, "runtime_profiles_and_resource_limits", "observations_reservation_uses_complete_proposal_and_preserves_prior_lines"),
]
MUTATIONS += VAL59_MUTATIONS

VAL60_MUTATIONS = [
    (
        "z60_flowglobal_alias",
        "src/ir/lower.rs",
        '"f0-0002-b-test-flow-global-32-v2" => Self::FlowCompilation32,',
        '"f0-0002-b-test-flow-global-32-v2" => Self::FlowFunction32,',
        0,
        "analysis_profiles_and_quotas",
        "analysis_quota_profiles_are_selected_by_their_cli_identifier",
    ),
    (
        "a60_lw_flow32_to_global",
        "src/ir/lower.rs",
        '"f0-0002-b-test-flow-32-v2" => Self::FlowFunction32,',
        '"f0-0002-b-test-flow-32-v2" => Self::FlowCompilation32,',
        0,
        "analysis_profiles_and_quotas",
        "analysis_quota_profiles_are_selected_by_their_cli_identifier",
    ),
    (
        "a60_lw_flow32_to_normal",
        "src/ir/lower.rs",
        '"f0-0002-b-test-flow-32-v2" => Self::FlowFunction32,',
        '"f0-0002-b-test-flow-32-v2" => Self::Normal,',
        0,
        "analysis_profiles_and_quotas",
        "analysis_quota_profiles_are_selected_by_their_cli_identifier",
    ),
    (
        "a60_lw_types32_to_normal",
        "src/ir/lower.rs",
        '"f0-0002-b-test-types-32-v2" => Self::Types32,',
        '"f0-0002-b-test-types-32-v2" => Self::Normal,',
        0,
        "analysis_profiles_and_quotas",
        "analysis_quota_profiles_are_selected_by_their_cli_identifier",
    ),
    (
        "a60_lw_effects32_to_normal",
        "src/ir/lower.rs",
        '"f0-0002-b-test-effects-32-v2" => Self::Effects32,',
        '"f0-0002-b-test-effects-32-v2" => Self::Normal,',
        0,
        "analysis_profiles_and_quotas",
        "analysis_quota_profiles_are_selected_by_their_cli_identifier",
    ),
    (
        "z60_main_input_profile_drop",
        "src/main.rs",
        "diagnostic.profile_id = profile.to_owned();",
        "let _ = profile;",
        0,
        "analysis_profiles_and_quotas",
        "selected_profile_is_preserved_on_input_and_mock_environment_failures",
    ),
    (
        "z60_main_input_view_truncated",
        "src/main.rs",
        "return diagnostic_result_view(\n            file,\n            view,\n            profile,",
        'return diagnostic_result_view(\n            file,\n            SourceView { bytes_len, text: Some("") },\n            profile,',
        0,
        "analysis_profiles_and_quotas",
        "selected_profile_is_preserved_on_input_and_mock_environment_failures",
    ),
]
MUTATIONS += VAL60_MUTATIONS

VAL61_MUTATIONS = [
    (
        "v61_main_mock_dash_value",
        "src/main.rs",
        '!args[i + 1].to_string_lossy().starts_with("--")',
        "!args[i + 1].to_string_lossy().starts_with('-')",
        2,
        "run_argv_dash_value_semantics",
        "single_dash_mock_environment_values_are_paths",
    ),
    (
        "v61_main_mock_contains_dashdash",
        "src/main.rs",
        '!args[i + 1].to_string_lossy().starts_with("--")',
        '!args[i + 1].to_string_lossy().contains("--")',
        2,
        "run_argv_dash_value_semantics",
        "embedded_double_dash_mock_environment_value_is_a_path",
    ),
    (
        "v61_main_profile_single_dash",
        "src/main.rs",
        '!args[i + 1].to_string_lossy().starts_with("--")',
        "!args[i + 1].to_string_lossy().starts_with('-')",
        0,
        "run_argv_dash_value_semantics",
        "single_dash_profile_value_reaches_profile_selection",
    ),
    (
        "v61_main_profile_contains_dashdash",
        "src/main.rs",
        '!args[i + 1].to_string_lossy().starts_with("--")',
        '!args[i + 1].to_string_lossy().contains("--")',
        0,
        "run_argv_dash_value_semantics",
        "embedded_double_dash_profile_value_reaches_profile_selection",
    ),
]
MUTATIONS += VAL61_MUTATIONS

VAL62_MUTATIONS = []
VALUE_GUARDS = [
    (
        "mock",
        [
            ("single", "!args[i + 1].to_string_lossy().starts_with('-')", "run_argv_dash_value_semantics", "single_dash_mock_environment_values_are_paths"),
            ("singlestr", '!args[i + 1].to_string_lossy().starts_with("-")', "run_argv_dash_value_semantics", "single_dash_mock_environment_values_are_paths"),
            ("contains", '!args[i + 1].to_string_lossy().contains("--")', "run_argv_dash_value_semantics", "embedded_double_dash_mock_environment_value_is_a_path"),
            ("exact", 'args[i + 1].to_string_lossy() != "--"', "run_argv_matrix", "generated_run_cli_argv_matrix_matches_d5_d15_a2_oracle"),
            ("empty", '!args[i + 1].to_string_lossy().starts_with("--") && !args[i + 1].is_empty()', "cli_argument_and_unicode_edges", "empty_mock_environment_value_is_a_path"),
            ("triple", '!args[i + 1].to_string_lossy().starts_with("---")', "run_argv_matrix", "generated_run_cli_argv_matrix_matches_d5_d15_a2_oracle"),
            ("none", "true", "run_argv_matrix", "generated_run_cli_argv_matrix_matches_d5_d15_a2_oracle"),
        ],
    ),
    (
        "profile",
        [
            ("single", "!args[i + 1].to_string_lossy().starts_with('-')", "run_argv_dash_value_semantics", "single_dash_profile_value_reaches_profile_selection"),
            ("contains", '!args[i + 1].to_string_lossy().contains("--")', "run_argv_dash_value_semantics", "embedded_double_dash_profile_value_reaches_profile_selection"),
            ("exact", 'args[i + 1].to_string_lossy() != "--"', "run_argv_matrix", "generated_run_cli_argv_matrix_matches_d5_d15_a2_oracle"),
            ("empty", '!args[i + 1].to_string_lossy().starts_with("--") && !args[i + 1].is_empty()', "cli_argument_and_unicode_edges", "empty_profile_value_reaches_profile_selection"),
            ("triple", '!args[i + 1].to_string_lossy().starts_with("---")', "run_argv_matrix", "generated_run_cli_argv_matrix_matches_d5_d15_a2_oracle"),
            ("none", "true", "run_argv_matrix", "generated_run_cli_argv_matrix_matches_d5_d15_a2_oracle"),
        ],
    ),
    (
        "format",
        [
            ("single", "!args[i + 1].to_string_lossy().starts_with('-')", "run_argv_dash_value_semantics", "single_dash_format_value_is_invalid_arguments"),
            ("contains", '!args[i + 1].to_string_lossy().contains("--")', "run_argv_dash_value_semantics", "embedded_double_dash_format_value_is_invalid_arguments"),
            ("exact", 'args[i + 1].to_string_lossy() != "--"', "run_argv_matrix", "generated_run_cli_argv_matrix_matches_d5_d15_a2_oracle"),
            ("empty", '!args[i + 1].to_string_lossy().starts_with("--") && !args[i + 1].is_empty()', "cli_argument_and_unicode_edges", "empty_format_value_is_invalid_arguments"),
            ("triple", '!args[i + 1].to_string_lossy().starts_with("---")', "run_argv_matrix", "generated_run_cli_argv_matrix_matches_d5_d15_a2_oracle"),
            ("none", "true", "run_argv_matrix", "generated_run_cli_argv_matrix_matches_d5_d15_a2_oracle"),
        ],
    ),
]
for option, variants in VALUE_GUARDS:
    old = (
        '&& !args[i + 1].to_string_lossy().starts_with("--") =>\n'
        "                {\n"
        f"                    seen_{option} = true;"
    )
    for variant, predicate, target, test in variants:
        if option == "format":
            continue
        new = f"&& {predicate} =>\n                {{\n                    seen_{option} = true;"
        VAL62_MUTATIONS.append(
            (f"v62_dash_{option}_{variant}", "src/main.rs", old, new, 0, target, test)
        )

VAL62_MUTATIONS += [
    ("v62_rep_profile_nocheck", "src/main.rs", "if !seen_profile\n", "if true\n", 0, "run_cli_diagnostics_and_cells", "repeated_options_and_option_names_used_as_values_are_usage_errors"),
    ("v62_rep_format_nocheck", "src/main.rs", "if !seen_format\n", "if true\n", 0, "run_cli_diagnostics_and_cells", "repeated_options_and_option_names_used_as_values_are_usage_errors"),
    ("v62_rep_mock_nocheck", "src/main.rs", "&& !seen_mock\n", "&& true\n", 0, "run_cli_diagnostics_and_cells", "repeated_options_and_option_names_used_as_values_are_usage_errors"),
    ("v62_rep_profile_nomark", "src/main.rs", "seen_profile = true;\n", "", 0, "run_cli_diagnostics_and_cells", "repeated_options_and_option_names_used_as_values_are_usage_errors"),
    ("v62_rep_format_nomark", "src/main.rs", "seen_format = true;\n", "", 0, "run_cli_diagnostics_and_cells", "repeated_options_and_option_names_used_as_values_are_usage_errors"),
    ("v62_rep_mock_nomark", "src/main.rs", "seen_mock = true;\n", "", 0, "run_cli_diagnostics_and_cells", "repeated_options_and_option_names_used_as_values_are_usage_errors"),
    ("v62_outer_single_dash", "src/main.rs", 'if arg.starts_with("--") {', "if arg.starts_with('-') {", 0, "cli_argument_and_unicode_edges", "single_dash_source_paths_are_sources"),
    ("v62_outer_triple", "src/main.rs", 'if arg.starts_with("--") {', 'if arg.starts_with("---") {', 0, "cli_argument_and_unicode_edges", "options_may_precede_or_follow_the_source"),
    ("v62_outer_dashdash_len", "src/main.rs", 'if arg.starts_with("--") {', 'if arg.starts_with("--") && arg.len() > 2 {', 0, "cli_argument_and_unicode_edges", "bare_double_dash_is_not_a_source_path_or_end_of_options"),
    ("v62_unknown_option_ignored", "src/main.rs", "                _ => return None,\n            }\n        } else if", "                _ => {}\n            }\n        } else if", 0, "run_argv_matrix", "generated_run_cli_argv_matrix_matches_d5_d15_a2_oracle"),
    ("v62_two_files_last_wins", "src/main.rs", "} else if file.replace(arg).is_some() {\n            return None;\n        }", "} else {\n            file.replace(arg);\n        }", 0, "run_argv_matrix", "generated_run_cli_argv_matrix_matches_d5_d15_a2_oracle"),
    ("v62_command_prefix_run", "src/main.rs", 'if command != "check" && command != "run" {', 'if command != "check" && !command.starts_with("run") {', 0, "cli_argument_and_unicode_edges", "only_exact_command_names_are_accepted"),
    ("v62_check_accepts_mock", "src/main.rs", '"--mock-env"\n                    if command == "run"\n                        && !seen_mock', '"--mock-env"\n                    if true\n                        && !seen_mock', 0, "cli", "cli_mock_environment_option_is_run_only_and_reports_exact_schema_reason"),
    ("v62_profile_lowercase", "src/main.rs", "profile = args[i].to_str()?.to_owned();", "profile = args[i].to_str()?.to_lowercase();", 0, "cli_argument_and_unicode_edges", "profile_identifiers_are_case_sensitive"),
    ("v62_format_empty_ok", "src/main.rs", 'if format != "text" && format != "json" {', 'if format != "text" && format != "json" && !format.is_empty() {', 0, "cli_argument_and_unicode_edges", "empty_format_value_is_invalid_arguments"),
    ("v62_profile_last_oob", "src/main.rs", '"--profile"\n                    if !seen_profile\n                        && i + 1 < args.len()', '"--profile"\n                    if !seen_profile\n                        && i + 1 <= args.len()', 0, "run_argv_matrix", "generated_run_cli_argv_matrix_matches_d5_d15_a2_oracle"),
    ("v62_format_last_oob", "src/main.rs", '"--format"\n                    if !seen_format\n                        && i + 1 < args.len()', '"--format"\n                    if !seen_format\n                        && i + 1 <= args.len()', 0, "run_argv_matrix", "generated_run_cli_argv_matrix_matches_d5_d15_a2_oracle"),
    ("v62_mock_last_oob", "src/main.rs", '&& !seen_mock\n                        && i + 1 < args.len()', '&& !seen_mock\n                        && i + 1 <= args.len()', 0, "run_argv_matrix", "generated_run_cli_argv_matrix_matches_d5_d15_a2_oracle"),
    ("v62_order_profile_needs_nofile", "src/main.rs", 'if !seen_profile\n                        && i + 1 < args.len()', 'if !seen_profile\n                        && file.is_none()\n                        && i + 1 < args.len()', 0, "run_argv_matrix", "generated_run_cli_argv_matrix_matches_d5_d15_a2_oracle"),
    ("v62_order_format_needs_nomock", "src/main.rs", 'if !seen_format\n                        && i + 1 < args.len()', 'if !seen_format\n                        && mock.is_none()\n                        && i + 1 < args.len()', 0, "run_argv_matrix", "generated_run_cli_argv_matrix_matches_d5_d15_a2_oracle"),
    ("v62_order_mock_needs_file", "src/main.rs", '&& !seen_mock\n                        && i + 1 < args.len()', '&& !seen_mock\n                        && file.is_some()\n                        && i + 1 < args.len()', 0, "run_argv_matrix", "generated_run_cli_argv_matrix_matches_d5_d15_a2_oracle"),
    ("v62_env_highsurr_range", "src/interp/mock_env.rs", "(0xD800..=0xDBFF).contains(&first)", "(0xD800..=0xDBFE).contains(&first)", 0, "mock_oracle", "every_unicode_code_unit_escape_has_the_contract_rejection_phase_in_keys_and_values"),
]
MUTATIONS += VAL62_MUTATIONS

DEBT_MUTATIONS = {"v55_order_work_cells"}

MUTATION_DIMENSIONS = {
    "mock_read_records": ["v55_records_reversed", "v55_records_drop_last", "n_read_event_ok_len", "v55_read_err_payload_plus1", "v55_read_err_event_abs", "n_read_pop_clear", "p_read_ok_work", "p_read_ok_cells", "read_empty_code"],
    "mock_write_i64": ["n_write_event_number", "v55_stdout_abs", "v55_write_err_event_abs", "write_error_event_ok", "n_write_no_pop", "p_write_work", "n_write_cells_1", "write_empty_code"],
    "mock_request": ["p_request_event", "v55_request_err_event_abs", "request_error_event_ok", "n_request_no_pop", "request_work_3", "request_cells_1", "request_empty_code"],
    "mock_reply": ["v55_reply_status_abs", "p_reply_err_event", "n_reply_no_pop", "reply_work_3", "reply_cells_1", "reply_empty_code"],
    "mock_configuration": ["v55_canon_plus", "v55_literal_false", "v55_utf8_reason", "v55_dup_last_only", "v55_main_unreadable_reason", "v55_order_size_utf8"],
}
MUTATION_AXES = {
    "order": ["read_queue_back", "write_queue_back", "request_back", "reply_queue_back", "batch_order", "v55_records_reversed"],
    "sign": ["n_write_event_number", "p_request_event", "v55_stdout_abs", "v55_reply_status_abs", "v55_read_err_event_abs", "v55_write_err_event_abs", "v55_request_err_event_abs"],
    "value": ["v55_records_drop_last", "v55_read_err_payload_plus1", "v55_unit_err_payload_neg", "into_record_ok_payload", "into_record_err_payload", "into_scalar_ok_false", "into_unit_err_true"],
    "length": ["n_read_event_ok_len", "read_event_count", "read_event_id_plus1", "v55_records_drop_last"],
    "rejection_reason": ["v55_canon_plus", "v55_literal_false", "v55_utf8_reason", "v55_main_unreadable_reason", "cli_env_too_large", "cli_schema_reason"],
    "validation_phase": ["v55_dup_last_only", "v55_order_size_utf8", "env_size_limit", "v55_utf8_reason"],
    "work_cost": ["p_read_ok_work", "read_work_plus1", "write_work_const", "request_work_3", "reply_work_3", "n_read_err_work3"],
    "cell_cost": ["p_read_ok_cells", "read_cells_x1", "request_cells_1", "reply_cells_1", "n_write_cells_1", "n_read_err_cells3"],
    "queue_consumption": ["n_read_pop_clear", "n_write_no_pop", "n_request_no_pop", "n_reply_no_pop", "read_empty_code", "write_empty_code", "request_empty_code", "reply_empty_code"],
}
MUTATION_DIMENSIONS["drop_result_cell_order"] = ["w58_drop_reserve_before_dispose"]
MUTATION_AXES["drop_result_cell_order"] = ["w58_drop_reserve_before_dispose"]

MUTATION_DIMENSIONS["mock_read_records"] += [
    "m56_read_empty_not_consumed",
    "m56_read_event_ok_records_only",
    "m56_read_cells_ok_records_only",
    "m56_read_work_ok_records_only",
    "m56_read_event_resource_zero",
]
MUTATION_DIMENSIONS["mock_write_i64"] += [
    "m56_write_err_not_consumed",
    "m56_write_stdout_plus",
    "m56_write_err_swapped",
    "m56_write_work_inverted",
]
MUTATION_DIMENSIONS["mock_request"] += [
    "m56_request_err_not_consumed",
    "m56_request_exhausted_ok0",
]
MUTATION_DIMENSIONS["mock_reply"] += [
    "m56_reply_err_consumes_two",
    "m56_reply_err_swapped",
    "m56_reply_exhausted_ok",
]
MUTATION_DIMENSIONS["mock_configuration"] += [
    "m56_dup_during_parse",
    "m56_trailing_one_byte_ok",
    "m56_trailing_schema",
    "m56_depth_schema",
    "m56_unterminated_string_schema",
    "m56_control_in_string_schema",
    "m56_canon_i64_saturate",
    "m56_version_any_number",
    "m56_result_any_key_err",
    "m56_record_value_zero",
    "m56_literal_true_trunc",
    "m56_hex_decimal_only",
    "m56_ws_formfeed",
    "m56_del_rejected",
    "m56_array_trailing_comma",
    "m56_object_trailing_comma",
    "m56_surrogate_start",
    "m56_exact5_dropped",
    "m56_leading_zero_len2",
    "m56_neg_zero_00",
    "m56_unreadable_text_flag",
    "m56_driver_error_exit1",
    "m56_too_large_reason_unreadable",
    "m56_read_err_inner_json",
    "m56_json_reason_as_schema",
]
MUTATION_AXES["order"] += [
    "m56_request_err_not_consumed",
    "m56_write_err_not_consumed",
    "m56_read_empty_not_consumed",
    "m56_reply_err_consumes_two",
]
MUTATION_AXES["sign"] += ["m56_reply_err_swapped", "m56_write_err_swapped"]
MUTATION_AXES["value"] += [
    "m56_record_value_zero",
    "m56_request_exhausted_ok0",
    "m56_reply_exhausted_ok",
]
MUTATION_AXES["length"] += ["m56_read_event_ok_records_only", "m56_read_event_resource_zero"]
MUTATION_AXES["rejection_reason"] += [
    "m56_unreadable_text_flag",
    "m56_too_large_reason_unreadable",
    "m56_json_reason_as_schema",
]
MUTATION_AXES["validation_phase"] += [
    "m56_dup_during_parse",
    "m56_trailing_one_byte_ok",
    "m56_trailing_schema",
    "m56_depth_schema",
    "m56_unterminated_string_schema",
    "m56_control_in_string_schema",
    "m56_ws_formfeed",
    "m56_del_rejected",
    "m56_array_trailing_comma",
    "m56_object_trailing_comma",
    "m56_read_err_inner_json",
]
MUTATION_AXES["work_cost"] += ["m56_read_work_ok_records_only", "m56_write_work_inverted"]
MUTATION_AXES["cell_cost"] += ["m56_read_cells_ok_records_only"]
MUTATION_AXES["queue_consumption"] += [
    "m56_request_err_not_consumed",
    "m56_write_err_not_consumed",
    "m56_read_empty_not_consumed",
    "m56_reply_err_consumes_two",
]
MUTATION_AXES["mock_parser_branch"] = [
    "m56_trailing_one_byte_ok",
    "m56_unterminated_string_schema",
    "m56_result_any_key_err",
    "m56_ws_formfeed",
    "m56_del_rejected",
    "m56_array_trailing_comma",
    "m56_object_trailing_comma",
]
MUTATION_AXES["main_driver_format_branch"] = [
    "m56_unreadable_text_flag",
    "m56_driver_error_exit1",
    "m56_too_large_reason_unreadable",
    "m56_read_err_inner_json",
    "m56_json_reason_as_schema",
]
MUTATION_AXES["type_and_verify_branch"] = [
    "m56_check_write_untyped",
    "m56_verify_request_type",
]
MUTATION_DIMENSIONS["mock_read_records"] += ["n57_read_empty_cells1"]
MUTATION_DIMENSIONS["mock_configuration"] += [
    "n57_frac_no_digit_ok",
    "n57_exp_no_digit_ok",
    "n57_exp_sign_plus_rejected",
]
MUTATION_DIMENSIONS["main_driver_format_branch"] = [
    "m56_unreadable_text_flag",
    "m56_driver_error_exit1",
    "m56_too_large_reason_unreadable",
    "m56_read_err_inner_json",
    "m56_json_reason_as_schema",
    "n57_json_fault_exit_success",
    "n57_text_fault_exit_success",
]
MUTATION_DIMENSIONS["cli_argument_branch"] = [
    "n57_format_repeat_ok",
    "n57_profile_repeat_ok",
    "n57_mock_value_dashdash_ok",
    "n57_invalid_format_accepted",
    "n57_unknown_option_ignored",
]
MUTATION_AXES["cell_cost"] += ["n57_read_empty_cells1"]
MUTATION_AXES["validation_phase"] += [
    "n57_frac_no_digit_ok",
    "n57_exp_no_digit_ok",
    "n57_exp_sign_plus_rejected",
]
MUTATION_AXES["mock_parser_numbers"] = [
    "n57_frac_no_digit_ok",
    "n57_exp_no_digit_ok",
    "n57_exp_sign_plus_rejected",
]
MUTATION_AXES["argument_cardinality"] = [
    "n57_format_repeat_ok",
    "n57_profile_repeat_ok",
    "n57_mock_value_dashdash_ok",
    "n57_invalid_format_accepted",
    "n57_unknown_option_ignored",
]
MUTATION_AXES["exit_status"] = [
    "n57_json_fault_exit_success",
    "n57_text_fault_exit_success",
]
MUTATION_AXES["unresolved_normative_observations"] = [
    "D2_work_cells_precedence",
    "D3_write_reply_argument_liveness",
]
MUTATION_DIMENSIONS["runtime_profile_limits"] = [
    "w58_limit_cells_normal",
    "w58_limit_obs_normal",
    "w58_limit_frames_normal",
    "w58_limit_work_normal",
    "profile_types32_limit",
    "w58_limit_cells64",
    "w58_limit_obs64",
    "w58_limit_frames8",
    "w58_profile_effects32_id",
    "w58_normal_work_minus1",
    "w58_normal_frames_minus1",
    "w58_normal_cells_plus1",
    "w58_normal_obs_minus1",
    "w58_frames8_minus1",
    "w58_cells64_minus1",
    "w58_obs64_minus1",
    "w58_types32_minus1",
    "w58_flow32_plus1",
    "w58_flow32_minus1",
    "w58_flow_global32_plus1",
    "w58_flow_global32_minus1",
    "w58_effects32_plus1",
    "w58_effects32_minus1",
    "w58_ir32_plus1",
    "w58_ir32_minus1",
    "w58_normal_types_minus1",
    "w58_normal_types_plus1",
    "w58_normal_flow_function_minus1",
    "w58_normal_flow_function_plus1",
    "w58_normal_flow_global_minus1",
    "w58_normal_flow_global_plus1",
    "w58_normal_effects_minus1",
    "w58_normal_effects_plus1",
    "w58_normal_ir_items_minus1",
    "w58_normal_ir_items_plus1",
]
MUTATION_DIMENSIONS["resource_cleanup_at_entry"] = [
    "w58_initial_entry_skips_resource_cleanup",
    "w58_drop_entry_skips_resource_cleanup",
]
MUTATION_DIMENSIONS["runtime_diagnostic_fields"] = [
    "w58_main_diag_profile_default",
    "w58_main_diag_before_obs",
    "w58_main_json_bytes_swap",
    "w58_main_err_json_kind",
    "w58_log_reserve_ge",
    "w58_log_emit_reserved_no_release",
]
MUTATION_DIMENSIONS["intrinsic_effects"] = [
    "w58_effects_write_pure",
    "w58_effects_reply_pure",
    "w58_effects_read_no_io",
    "w58_effects_open_pure",
]
MUTATION_DIMENSIONS["val58_production_sites"] = [item[0] for item in W58_MUTATIONS if item[1].startswith("src/")]
MUTATION_DIMENSIONS["profile_document_ids"] = ["w58_catalog_profile_old_id"]
MUTATION_DIMENSIONS["cli_profile_and_format"] = [
    "w58_main_unknown_profile_text",
    "w58_main_check_json_exit",
    "w58_main_explain_err_exit1",
    "w58_main_explain_ok_exit1",
    "w58_main_ok_text_stdout",
    "w58_main_profile_default",
    "w58_main_format_default",
    "w58_main_profile_value_ignored",
    "w58_main_format_value_ignored",
]
MUTATION_DIMENSIONS["runtime_diagnostic_fields"] += [
    "w58_r_profile_defaults",
    "w58_r_work_limit_plus1",
    "w58_r_work_observed_plus1",
    "w58_r_work_consumed_plus1",
    "w58_r_frames_observed_plus1",
    "w58_r_cells_observed_plus1",
    "w58_r_observation_observed_plus1",
]
MUTATION_AXES["runtime_diagnostic_field_isolation"] = MUTATION_DIMENSIONS[
    "runtime_diagnostic_fields"
]

MUTATION_DIMENSIONS["val59_measured_prior_slice_replay"] = [
    item[0] for item in VAL59_MUTATIONS if item[0].startswith(("x59_", "y59_"))
]
MUTATION_DIMENSIONS["val59_i11a_acquisition_proposal"] = [
    "v59_open_acquisition_observed"
]
MUTATION_AXES["val59_measured_prior_slice_replay"] = MUTATION_DIMENSIONS[
    "val59_measured_prior_slice_replay"
]
MUTATION_AXES["val59_i11a_acquisition_proposal"] = MUTATION_DIMENSIONS[
    "val59_i11a_acquisition_proposal"
]
MUTATION_DIMENSIONS["val60_profile_wiring"] = [item[0] for item in VAL60_MUTATIONS]
MUTATION_AXES["val60_profile_wiring"] = MUTATION_DIMENSIONS["val60_profile_wiring"]
MUTATION_DIMENSIONS["cli_argument_dash_boundary"] = [
    "v61_main_mock_dash_value",
    "v61_main_mock_contains_dashdash",
    "v61_main_profile_single_dash",
    "v61_main_profile_contains_dashdash",
    "v61_main_format_single_dash",
    "v61_main_format_contains_dashdash",
]
MUTATION_AXES["cli_argument_dash_boundary"] = MUTATION_DIMENSIONS[
    "cli_argument_dash_boundary"
]
MUTATION_DIMENSIONS["cli_run_parse_surface"] = [item[0] for item in VAL62_MUTATIONS if item[1] == "src/main.rs"]
MUTATION_AXES["cli_run_parse_surface"] = MUTATION_DIMENSIONS["cli_run_parse_surface"]
MUTATION_DIMENSIONS["val60_input_diagnostic_view"] = [
    "z60_main_input_view_truncated"
]
MUTATION_AXES["input_diagnostic_view"] = MUTATION_DIMENSIONS[
    "val60_input_diagnostic_view"
]
MUTATION_AXES["cli_profile_selection"] = MUTATION_DIMENSIONS["cli_profile_and_format"]
MUTATION_AXES["all_val58_directed_sites"] = MUTATION_DIMENSIONS[
    "val58_production_sites"
]
MUTATION_AXES["profile_document_ids"] = MUTATION_DIMENSIONS["profile_document_ids"]
MUTATION_AXES["runtime_profile_boundary"] = MUTATION_DIMENSIONS["runtime_profile_limits"]
MUTATION_AXES["resource_temporary_entry_cleanup"] = MUTATION_DIMENSIONS["resource_cleanup_at_entry"]
MUTATION_AXES["runtime_diagnostic_field_and_channel"] = MUTATION_DIMENSIONS["runtime_diagnostic_fields"]
MUTATION_AXES["intrinsic_effect_checks"] = MUTATION_DIMENSIONS["intrinsic_effects"]
MUTATION_AXES["cli_profile_format_and_explain"] = MUTATION_DIMENSIONS["cli_profile_and_format"]

CARGO = shutil.which("cargo") or str(Path.home() / ".cargo" / "bin" / "cargo.exe")
MINIMUM_OBSERVED_FREE_BYTES = shutil.disk_usage(ROOT).free

EQUIVALENTS = [
    (
        "indexvalue_fault_plus1",
        "src/interp/machine.rs",
        "La rama de bounds fault termina inmediatamente en R0102; el contador de celdas no se emite y la limpieza posterior usa saturating_sub. La regresión de índice temporal comprueba el código y la limpieza.",
        ("self.cells = self.cells.saturating_sub(size + own);", "self.cells = self.cells.saturating_sub(size + 1);", 0, "all", ""),
    ),
    (
        "write_index_always_dec",
        "src/interp/machine.rs",
        "WriteIndex recibe el selector propietario producido por self.expr; la prueba de escritura indexada y la batería completa ejercitan su frontera.",
        ("if matches!(taken, TempValue::Own(_)) {\n                                self.cells = self.cells.saturating_sub(1);\n                            }\n                            path.push(*n);", "{\n                                self.cells = self.cells.saturating_sub(1);\n                            }\n                            path.push(*n);", 1, "all", ""),
    ),
    (
        "lower_write_ty",
        "src/ir/lower.rs",
        "ParameterMode::Read frente a Consume para I64 no cambia el escalar ni sus alias; la prueba B completa de MockWriteI64 y la suite verifican su ruta.",
        ("Intrinsic::MockWriteI64 => vec![(\"n\".into(), ParameterMode::Read, Ty::I64)],", "Intrinsic::MockWriteI64 => vec![(\"n\".into(), ParameterMode::Consume, Ty::I64)],", 0, "mock_oracle", "mock_oracle_runtime_contract"),
    ),
    (
        "check_type_limit",
        "src/check.rs",
        "El límite de un millón de pasos de la entrada pública no puede alcanzarse dentro del límite de fuente de 1 MiB y la profundidad sintáctica máxima; la suite completa ejecuta los perfiles públicos.",
        ("check_ast_with_worker_failure(file, source, &ast, force_unavailable, 1_000_000)", "check_ast_with_worker_failure(file, source, &ast, force_unavailable, 999_999)", 0, "all", ""),
    ),
    (
        "env_depth_eq",
        "src/interp/mock_env.rs",
        "enter() comprueba el valor actual antes de incrementar; cualquier llamada con depth == 16 se rechaza y ninguna ruta válida puede hacer depth > 16. Por ello >= 16 y == 16 tienen el mismo resultado para toda entrada.",
        ("if self.depth >= 16 {", "if self.depth == 16 {", 0, "mock_oracle", "mock_oracle_parser_contract"),
    ),
    (
        "env_no_leading_zero_guard",
        "src/interp/mock_env.rs",
        "El número consume un único cero; si se omite esta guarda, el segundo dígito queda como token sobrante y parse() lo rechaza igualmente como invalid_mock_json. No existe un sufijo que haga válido el número JSON con cero inicial.",
        ("if self.peek().is_some_and(|b| b.is_ascii_digit()) {", "if false {", 0, "mock_oracle", "mock_oracle_parser_contract"),
    ),
    (
        "x59_r0103_message",
        "src/interp/machine.rs",
        "A2 §13.6 l. 1893 define `message` como presentación que no identifica la regla; A2 §13.8 l. 2160 permite variar su traducción. Código, operation, span y campos estructurados no cambian.",
        ('"R0103" => "Desbordamiento de resta i64.",', '"R0103" => "Desbordamiento de suma i64.",', 0, "runtime_profiles_and_resource_limits", "runtime_profile_table_drives_work_frames_cells_and_observation_boundaries"),
    ),
    (
        "v55_order_size_utf8",
        "src/interp/mock_env.rs",
        "A2 §12.4, «Esquema del entorno», l. 1538 y F0-0004 §6.1 l. 229: el CLI ejecuta read_bounded antes de MockEnvironment::parse. El probe crea un archivo mayor de 1 MiB que empieza con 0xFF y comprueba el JSON exacto mock_env_too_large; ambas órdenes internas producen la misma salida CLI.",
        (
            """        if bytes.len() > 1_048_576 {
            return Err(MockEnvironmentError::TooLarge);
        }
        let text = std::str::from_utf8(bytes).map_err(|_| MockEnvironmentError::InvalidJson)?;""",
            """        let text = std::str::from_utf8(bytes).map_err(|_| MockEnvironmentError::InvalidJson)?;
        if bytes.len() > 1_048_576 {
            return Err(MockEnvironmentError::TooLarge);
        }""",
            0,
            "mock_env_records_and_failures",
            "cli_size_limit_precedes_invalid_utf8_for_mock_environment",
        ),
    ),
    (
        "a60_lw_frames8_to_normal",
        "src/ir/lower.rs",
        "`frames-8` changes only the runtime call-frame quota, not any analysis quota. `ReducedProfile` is used by lowering; the CLI overlays the chosen profile identity on analysis diagnostics, while execution limits are selected independently by `ReducedExecutionProfile::from_id`. Mapping this analysis row to Normal therefore preserves every observable compiler and CLI result.",
        (
            '"f0-0002-b-test-frames-8-v2" => Self::Frames8,',
            '"f0-0002-b-test-frames-8-v2" => Self::Normal,',
            0,
            "analysis_profiles_and_quotas",
            "analysis_quota_profiles_are_selected_by_their_cli_identifier",
        ),
    ),
    (
        "a60_lw_cells64_to_normal",
        "src/ir/lower.rs",
        "`cells-64` changes only the runtime live-cell quota, not any analysis quota. Lowering uses `ReducedProfile`; runtime selects its limits independently from the same CLI identifier, and `main` restores the selected profile ID on analysis diagnostics. Mapping this analysis row to Normal is unobservable.",
        (
            '"f0-0002-b-test-cells-64-v2" => Self::Cells64,',
            '"f0-0002-b-test-cells-64-v2" => Self::Normal,',
            0,
            "analysis_profiles_and_quotas",
            "analysis_quota_profiles_are_selected_by_their_cli_identifier",
        ),
    ),
    (
        "a60_lw_obs64_to_normal",
        "src/ir/lower.rs",
        "`observations-64` changes only the runtime observation-byte quota, not any analysis quota. Lowering uses `ReducedProfile`; runtime selects its limits independently from the CLI identifier, and `main` restores the selected profile ID on analysis diagnostics. Mapping this analysis row to Normal is unobservable.",
        (
            '"f0-0002-b-test-observations-64-v2" => Self::Observations64,',
            '"f0-0002-b-test-observations-64-v2" => Self::Normal,',
            0,
            "analysis_profiles_and_quotas",
            "analysis_quota_profiles_are_selected_by_their_cli_identifier",
        ),
    ),
    (
        "v61_main_format_single_dash",
        "src/main.rs",
        "`--format` admite únicamente `text` o `json`; un valor `-x` es inválido en ambas rutas. Rechazarlo en la guarda de valor o después de asignarlo produce el mismo `invalid_arguments`, código de salida y salida literal para todo valor que comienza por un guion simple.",
        (
            '!args[i + 1].to_string_lossy().starts_with("--")',
            "!args[i + 1].to_string_lossy().starts_with('-')",
            1,
            "run_argv_dash_value_semantics",
            "single_dash_format_value_is_invalid_arguments",
        ),
    ),
    (
        "v61_main_format_contains_dashdash",
        "src/main.rs",
        "`--format` admite únicamente `text` o `json`; cualquier valor que contiene `--` es inválido en ambas rutas. Rechazarlo en la guarda de valor o después de asignarlo produce el mismo `invalid_arguments`, código de salida y salida literal.",
        (
            '!args[i + 1].to_string_lossy().starts_with("--")',
            '!args[i + 1].to_string_lossy().contains("--")',
            1,
            "run_argv_dash_value_semantics",
            "embedded_double_dash_format_value_is_invalid_arguments",
        ),
    ),
    (
        "v62_dash_format_single",
        "src/main.rs",
        "`--format` solo acepta `text` o `json`; un valor que empieza por un guion simple se rechaza como `invalid_arguments` después de consumirlo, igual que al no aceptarlo como valor. A2 §12.4 l. 1492 y D15 §7.1 l. 758 publican el conjunto cerrado de formatos.",
        (
            '&& !args[i + 1].to_string_lossy().starts_with("--") =>\n                {\n                    seen_format = true;',
            "&& !args[i + 1].to_string_lossy().starts_with('-') =>\n                {\n                    seen_format = true;",
            0,
            "run_argv_matrix",
            "format_option_rejects_values_outside_the_closed_text_json_pair",
        ),
    ),
    (
        "v62_dash_format_contains",
        "src/main.rs",
        "`--format` solo acepta `text` o `json`; todo texto que contiene `--` no pertenece al conjunto, por lo que ambas rutas devuelven `invalid_arguments`. A2 §12.4 l. 1492 y D15 §7.1 l. 758 publican el conjunto cerrado.",
        (
            '&& !args[i + 1].to_string_lossy().starts_with("--") =>\n                {\n                    seen_format = true;',
            '&& !args[i + 1].to_string_lossy().contains("--") =>\n                {\n                    seen_format = true;',
            0,
            "run_argv_matrix",
            "format_option_rejects_values_outside_the_closed_text_json_pair",
        ),
    ),
    (
        "v62_dash_format_exact",
        "src/main.rs",
        "El valor exacto `--` no es `text` ni `json`; la guarda previa y la validación posterior convergen en el mismo `invalid_arguments`.",
        (
            '&& !args[i + 1].to_string_lossy().starts_with("--") =>\n                {\n                    seen_format = true;',
            '&& args[i + 1].to_string_lossy() != "--" =>\n                {\n                    seen_format = true;',
            0,
            "run_argv_matrix",
            "format_option_rejects_values_outside_the_closed_text_json_pair",
        ),
    ),
    (
        "v62_dash_format_empty",
        "src/main.rs",
        "El formato vacío no pertenece al conjunto cerrado `text|json`; exigirlo en la guarda o rechazarlo inmediatamente después conserva la misma salida literal `invalid_arguments`.",
        (
            '&& !args[i + 1].to_string_lossy().starts_with("--") =>\n                {\n                    seen_format = true;',
            '&& !args[i + 1].to_string_lossy().starts_with("--") && !args[i + 1].is_empty() =>\n                {\n                    seen_format = true;',
            0,
            "run_argv_matrix",
            "format_option_rejects_values_outside_the_closed_text_json_pair",
        ),
    ),
    (
        "v62_dash_format_triple",
        "src/main.rs",
        "El valor `---` tampoco pertenece a `text|json`; su rechazo antes o después de consumirlo es observacionalmente idéntico.",
        (
            '&& !args[i + 1].to_string_lossy().starts_with("--") =>\n                {\n                    seen_format = true;',
            '&& !args[i + 1].to_string_lossy().starts_with("---") =>\n                {\n                    seen_format = true;',
            0,
            "run_argv_matrix",
            "format_option_rejects_values_outside_the_closed_text_json_pair",
        ),
    ),
    (
        "v62_dash_format_none",
        "src/main.rs",
        "Aceptar cualquier token como candidato a formato no amplía el conjunto: la comprobación posterior solo admite `text` y `json`, y para cada otro valor produce el mismo `invalid_arguments`.",
        (
            '&& !args[i + 1].to_string_lossy().starts_with("--") =>\n                {\n                    seen_format = true;',
            "&& true =>\n                {\n                    seen_format = true;",
            0,
            "run_argv_matrix",
            "format_option_rejects_values_outside_the_closed_text_json_pair",
        ),
    ),
    (
        "v62_d1_parse_toolarge_reason",
        "src/main.rs",
        "`read_bounded` impone el límite de 1.048.576 bytes antes de `MockEnvironment::parse` (A2 §12.4 ll. 1501 y 1538); para la CLI el resultado `MockEnvironmentError::TooLarge` es inalcanzable. El caso superior al límite se clasifica antes como `mock_env_too_large` y no distingue esta rama interna.",
        (
            'MockEnvironmentError::TooLarge) => {\n                return driver_error(Some(&profile_id), "mock_env_too_large"',
            'MockEnvironmentError::TooLarge) => {\n                return driver_error(Some(&profile_id), "invalid_mock_json"',
            0,
            "cli",
            "cli_reports_mock_environment_size_limit_reason",
        ),
    ),
]
for mutation_id, old, new in ESCAPE_MUTATIONS:
    EQUIVALENTS.append(
        (
            mutation_id,
            "src/interp/mock_env.rs",
            "El cambio altera un escape cuyo carácter decodificado no puede formar parte de ninguna de las cinco claves requeridas ni de un I64 decimal canónico. Las claves desconocidas y cualquier I64 no decimal ya son invalid_mock_schema; el escape no altera la sintaxis JSON. El test parser contract y la campaña incluyen estas posiciones.",
            (old, new, 0, "mock_oracle", "mock_oracle_parser_contract"),
        )
    )

DEBT_BASIS = {
    "v55_order_work_cells": "Deuda D2: A2 §11.6 ll. 1062–1064 fija trabajo y celdas por primitiva, pero no prioriza ambas comprobaciones ante doble agotamiento. El probe de 71.425 iteraciones, 40 celdas y lectura de 40 registros observa R0001 en producción y R0002 con el mutante; se registra como deuda normativa, no como equivalencia."
}

OBSERVATION_DEBTS = {
    "D3_write_reply_argument_liveness": {
        "basis": "A2 §11.6 ll. 1063–1064 fija el coste de las primitivas write/reply, pero no declara si el literal argumento permanece vivo al reservar el resultado; §11.6 ll. 1071–1073 prohíbe adelantar liberaciones de argumentos prestados, no determina el tiempo de vida de este literal. La prueba de cells-64 conserva el comportamiento actual sin atribuirlo al ADR.",
        "status": "normative_gap",
        "test": "run_cli_diagnostics_and_cells::cells64_write_and_reply_literal_arguments_keep_the_current_frontier",
    },
    "unknown_profile_json_profile_id": {
        "basis": "A2 §12.4 l. 1495 publica los perfiles y §12.4 l. 1631 incluye unknown_profile entre las razones driver_error; §12.4 l. 1606 exige profile_id como cadena, pero no prescribe si el error de selección conserva el identificador desconocido o usa el perfil predeterminado. El JSON actual conserva el texto no reconocido (p. ej. bogus).",
        "status": "normative_gap",
        "test": "runtime_profiles_and_resource_limits::profile_selection_matrix_covers_defaults_all_profiles_and_unknown_ids",
    }
}


def mutate(copy: Path, file: str, old: str, new: str, occurrence: int) -> tuple[bytes, bytes]:
    path = copy / file
    before = path.read_bytes()
    text = before.decode("utf-8")
    if text.count(old) <= occurrence:
        raise RuntimeError(f"NoMatch en {file}, aparición {occurrence}: {old!r}")
    start = -1
    for _ in range(occurrence + 1):
        start = text.index(old, start + 1)
    after_text = text[:start] + new + text[start + len(old) :]
    after = after_text.encode("utf-8")
    path.write_bytes(after)
    return before, after


def run_test(copy: Path, target: str, test: str) -> tuple[bool, str]:
    global MINIMUM_OBSERVED_FREE_BYTES
    args = [CARGO, "test", "--locked", "--offline"]
    if target == "all":
        pass
    elif target == "lib_all":
        args += ["--lib"]
    elif target == "lib":
        args += ["--lib", test, "--", "--exact"]
    elif target == "mock_oracle_ignored":
        args += ["--test", "mock_oracle", test, "--", "--ignored", "--exact", "--nocapture"]
    else:
        args += ["--test", target, test, "--", "--exact"]
    result = subprocess.run(args, cwd=copy, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, check=False)
    free = shutil.disk_usage(ROOT).free
    MINIMUM_OBSERVED_FREE_BYTES = min(MINIMUM_OBSERVED_FREE_BYTES, free)
    output = result.stdout.decode("utf-8", errors="replace")
    if (
        result.returncode == 0
        and target not in ("all", "lib_all")
        and not re.search(rf"^test {re.escape(test)} \.\.\. (?:ok|FAILED|ignored)$", output, re.M)
    ):
        raise RuntimeError(f"requested mutation test did not execute: {target}::{test}\n{output[-3000:]}")
    failed = result.returncode != 0 and (
        "test result: FAILED." in output
        if target in ("all", "lib_all")
        else f"test {test} ... FAILED" in output
    )
    return failed, output


def run_release(copy: Path, *arguments: str) -> tuple[int, str]:
    global MINIMUM_OBSERVED_FREE_BYTES
    command = [CARGO, "test", "--locked", "--offline", "--release", *arguments]
    result = subprocess.run(
        command,
        cwd=copy,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
    )
    free = shutil.disk_usage(ROOT).free
    MINIMUM_OBSERVED_FREE_BYTES = min(MINIMUM_OBSERVED_FREE_BYTES, free)
    return result.returncode, result.stdout.decode("utf-8", errors="replace")


def release_target_arguments(target: str, test: str) -> list[str]:
    if target == "all":
        return []
    if target == "lib_all":
        return ["--lib"]
    if target == "lib":
        return ["--lib", test, "--", "--exact"]
    if target == "mock_oracle_ignored":
        return ["--test", "mock_oracle", test, "--", "--ignored", "--exact", "--nocapture"]
    return ["--test", target, test, "--", "--exact"]


def verify_from_id_wiring_mutations(copy: Path) -> list[dict]:
    # Independent table transcription from A2 §11.4/§11.5 and D15 §6.1.
    rows = [
        ("f0-0002-b-v2", "Normal", "Types32"),
        ("f0-0002-b-test-types-32-v2", "Types32", "Normal"),
        ("f0-0002-b-test-flow-32-v2", "FlowFunction32", "FlowCompilation32"),
        ("f0-0002-b-test-flow-global-32-v2", "FlowCompilation32", "FlowFunction32"),
        ("f0-0002-b-test-effects-32-v2", "Effects32", "Normal"),
        ("f0-0002-b-test-ir-32-v2", "IrItems32", "Normal"),
        ("f0-0002-b-test-frames-8-v2", "Frames8", "Types32"),
        ("f0-0002-b-test-cells-64-v2", "Cells64", "Types32"),
        ("f0-0002-b-test-observations-64-v2", "Observations64", "Types32"),
    ]
    test = "analysis_quota_profiles_are_selected_by_their_cli_identifier"
    checks = []
    for profile_id, variant, replacement_variant in rows:
        original_line = f'"{profile_id}" => Self::{variant},'
        mutations = [
            ("id", original_line, f'"{profile_id}-mutated" => Self::{variant},'),
            (
                "variant",
                original_line,
                f'"{profile_id}" => Self::{replacement_variant},',
            ),
        ]
        for mutation_kind, old, new in mutations:
            path = copy / "src/ir/lower.rs"
            original = path.read_bytes()
            _, mutated = mutate(copy, "src/ir/lower.rs", old, new, 0)
            rc, output = run_release(
                copy,
                "--test",
                "analysis_profiles_and_quotas",
                test,
                "--",
                "--exact",
            )
            path.write_bytes(original)
            restored = path.read_bytes()
            failed_exact_test = re.search(rf"^test {re.escape(test)} \.\.\. FAILED$", output, re.M)
            compile_error = "error: could not compile" in output or "error[E" in output
            killed = rc != 0 and failed_exact_test is not None and not compile_error
            checks.append(
                {
                    "profile_id": profile_id,
                    "site": mutation_kind,
                    "status": "killed" if killed else "survived",
                    "test": f"analysis_profiles_and_quotas::{test}",
                    "production_sha256": sha(original),
                    "mutated_sha256": sha(mutated),
                    "restored_sha256": sha(restored),
                }
            )
            print(
                f"from_id {mutation_kind} {profile_id} killed={killed} restored={sha(original) == sha(restored)}",
                flush=True,
            )
            if not killed or failed_exact_test is None or compile_error or sha(original) != sha(restored):
                raise RuntimeError(
                    f"from_id wiring mutation survived or did not restore: {mutation_kind} {profile_id}\n{output[-3000:]}"
                )
    return checks


def run_quick_release_population() -> int:
    global MINIMUM_OBSERVED_FREE_BYTES
    if COPY.exists():
        shutil.rmtree(COPY)
    ignore = shutil.ignore_patterns(
        ".git", "target", ".i11a-mutation-copy", "__pycache__", "*scratch"
    )
    shutil.copytree(ROOT, COPY, ignore=ignore)
    records = []
    try:
        for mutation in MUTATIONS:
            mid, file, old, new, occurrence, target, test = mutation
            path = COPY / file
            original = path.read_bytes()
            baseline_probe = None
            baseline_diagnostic_json = None
            if mid in DEBT_MUTATIONS:
                rc, output = run_release(
                    COPY,
                    "--test",
                    "mock_oracle",
                    test,
                    "--",
                    "--ignored",
                    "--exact",
                    "--nocapture",
                )
                if rc:
                    raise SystemExit(f"release debt probe failed before mutation {mid}:\n{output[-4000:]}")
                match = re.search(r"work_cells_priority_probe=(R\d{4})", output)
                baseline_probe = match.group(1) if match else None
                match = re.search(r"json=(\{[^\r\n]*\})", output)
                baseline_diagnostic_json = match.group(1) if match else None

            before, mutated = mutate(COPY, file, old, new, occurrence)
            target_arguments = release_target_arguments(target, test)
            first_stage = "cargo test --release" + (
                f" --test {target} {test}" if target not in ("all", "lib_all", "lib") else ""
            )
            rc, output = run_release(COPY, *target_arguments)
            if rc == 0:
                first_stage = "cargo test --release --no-fail-fast confirmation"
                rc, output = run_release(COPY, "--no-fail-fast")
            compile_error = "error: could not compile" in output or "error[E" in output
            status = "compile_error" if compile_error else ("killed" if rc else "survived")
            record = {
                "id": mid,
                "status": status,
                "test": first_stage,
                "production_sha256": sha(before),
                "mutated_sha256": sha(mutated),
                "restored_sha256": "",
            }
            if mid in DEBT_MUTATIONS and status == "survived":
                probe_rc, probe_output = run_release(
                    COPY,
                    "--test",
                    "mock_oracle",
                    test,
                    "--",
                    "--ignored",
                    "--exact",
                    "--nocapture",
                )
                match = re.search(r"work_cells_priority_probe=(R\d{4})", probe_output)
                mutated_probe = match.group(1) if match else None
                match = re.search(r"json=(\{[^\r\n]*\})", probe_output)
                mutated_diagnostic_json = match.group(1) if match else None
                record.update(
                    {
                        "status": "debt" if probe_rc == 0 and mutated_probe == "R0002" else "survived",
                        "debt_basis": DEBT_BASIS[mid],
                        "baseline_probe": baseline_probe,
                        "mutated_probe": mutated_probe,
                        "baseline_diagnostic_json": baseline_diagnostic_json,
                        "mutated_diagnostic_json": mutated_diagnostic_json,
                    }
                )
            path.write_bytes(before)
            restored = path.read_bytes()
            record["restored_sha256"] = sha(restored)
            records.append(record)
            print(
                f"{mid} {record['status']} {first_stage} restored={record['production_sha256'] == record['restored_sha256']}",
                flush=True,
            )
            if (
                record["status"] not in ("killed", "debt")
                or record["production_sha256"] != record["restored_sha256"]
            ):
                break

        if len(records) == len(MUTATIONS):
            for equivalent in EQUIVALENTS:
                mid, file, basis, *probe = equivalent
                path = COPY / file
                original = path.read_bytes()
                record = {
                    "id": mid,
                    "status": "equivalent",
                    "equivalence_basis": basis,
                    "production_sha256": sha(original),
                    "restored_sha256": "",
                }
                if probe:
                    old, new, occurrence, target, test = probe[0]
                    before, mutated = mutate(COPY, file, old, new, occurrence)
                    target_arguments = release_target_arguments(target, test)
                    stage = "cargo test --release" + (
                        f" --test {target} {test}"
                        if target not in ("all", "lib_all", "lib")
                        else ""
                    )
                    rc, output = run_release(COPY, *target_arguments)
                    if rc == 0:
                        stage = "cargo test --release --no-fail-fast confirmation"
                        rc, output = run_release(COPY, "--no-fail-fast")
                    record.update(
                        {
                            "status": "equivalent" if rc == 0 else "killed",
                            "test": stage,
                            "mutated_sha256": sha(mutated),
                            "production_sha256": sha(before),
                        }
                    )
                    path.write_bytes(before)
                restored = path.read_bytes()
                record["restored_sha256"] = sha(restored)
                records.append(record)
                print(
                    f"{mid} {record['status']} restored={record['production_sha256'] == record['restored_sha256']}",
                    flush=True,
                )
                if record["status"] != "equivalent" or record["production_sha256"] != record["restored_sha256"]:
                    break

        complete_population = len(records) == len(MUTATIONS) + len(EQUIVALENTS) and all(
            item["status"] in ("killed", "equivalent", "debt")
            and item["production_sha256"] == item["restored_sha256"]
            for item in records
        )
        from_id_wiring_checks = (
            verify_from_id_wiring_mutations(COPY) if complete_population else []
        )

        payload = json.dumps(
            {
                "cut": "I11a",
                "population_source": "working_tree",
                "population_profile": "release_quick",
                "observability_dimensions": MUTATION_DIMENSIONS,
                "mutation_axes": MUTATION_AXES,
                "debt_mutations": sorted(DEBT_MUTATIONS),
                "observation_debts": OBSERVATION_DEBTS,
                "mutations": records,
                "from_id_wiring_checks": from_id_wiring_checks,
            },
            ensure_ascii=False,
            indent=2,
            sort_keys=True,
        ) + "\n"
        OUTPUT.write_bytes(payload.encode("utf-8"))
        expected = len(MUTATIONS) + len(EQUIVALENTS)
        allowed = ("killed", "equivalent", "debt")
        complete = len(records) == expected and all(
            item["status"] in allowed
            and item["production_sha256"] == item["restored_sha256"]
            for item in records
        )
        print(
            f"release_population={len(records)}/{expected} minimum_free_bytes={MINIMUM_OBSERVED_FREE_BYTES}",
            flush=True,
        )
        return 0 if complete else 1
    finally:
        shutil.rmtree(COPY, ignore_errors=True)


def main() -> int:
    if "--quick-release" in sys.argv[1:]:
        if len(sys.argv) != 2:
            raise SystemExit("--quick-release runs the complete population and takes no mutant IDs")
        return run_quick_release_population()
    requested = set(sys.argv[1:])
    resume = os.environ.get("NEXO_MUTATION_RESUME") == "1" and not requested
    known = {mutation[0] for mutation in MUTATIONS}
    if requested - known:
        raise SystemExit(f"unknown mutation ids: {sorted(requested - known)}")
    selected = [mutation for mutation in MUTATIONS if not requested or mutation[0] in requested]
    if COPY.exists():
        shutil.rmtree(COPY)
    ignore = shutil.ignore_patterns(
        ".git", "target", ".i11a-mutation-copy", "__pycache__", "*scratch"
    )
    shutil.copytree(ROOT, COPY, ignore=ignore)
    records = []
    try:
        resumable = {}
        if resume and OUTPUT.exists():
            prior = json.loads(OUTPUT.read_text(encoding="utf-8"))
            prior_by_id = {record["id"]: record for record in prior.get("mutations", [])}
            for mutation in selected:
                mid, file = mutation[0], mutation[1]
                previous = prior_by_id.get(mid)
                source_hash = sha((ROOT / file).read_bytes())
                if (
                    previous
                    and previous.get("status") == "killed"
                    and previous.get("production_sha256") == source_hash
                    and previous.get("restored_sha256") == source_hash
                ):
                    resumable[mid] = previous
        for mutation in selected:
            mid, file, old, new, occurrence, target, test = mutation
            if mid in resumable:
                records.append(resumable[mid])
                print(f"{mid} resumed-killed", flush=True)
                continue
            baseline_probe = None
            baseline_diagnostic_json = None
            if mid in DEBT_MUTATIONS:
                baseline_failed, baseline_output = run_test(COPY, target, test)
                if baseline_failed:
                    raise SystemExit(f"baseline debt probe failed: {mid}\n{baseline_output[-4000:]}")
                match = re.search(r"work_cells_priority_probe=(R\d{4})", baseline_output)
                baseline_probe = match.group(1) if match else None
                match = re.search(r"json=(\{[^\r\n]*\})", baseline_output)
                baseline_diagnostic_json = match.group(1) if match else None
            path = COPY / file
            original, mutated = mutate(COPY, file, old, new, occurrence)
            mutated_sha = sha(mutated)
            killed, output = run_test(COPY, target, test)
            path.write_bytes(original)
            restored = path.read_bytes()
            record = {
                "id": mid,
                "status": "killed" if killed else ("compile_error" if "error: could not compile" in output else "survived"),
                "test": f"{target}::{test}",
                "production_sha256": sha(original),
                "mutated_sha256": mutated_sha,
                "restored_sha256": sha(restored),
            }
            if not killed and mid in DEBT_MUTATIONS:
                record["status"] = "debt"
                record["debt_basis"] = DEBT_BASIS[mid]
                match = re.search(r"work_cells_priority_probe=(R\d{4})", output)
                record["baseline_probe"] = baseline_probe
                record["mutated_probe"] = match.group(1) if match else None
                match = re.search(r"json=(\{[^\r\n]*\})", output)
                record["baseline_diagnostic_json"] = baseline_diagnostic_json
                record["mutated_diagnostic_json"] = match.group(1) if match else None
                print(
                    f"{mid} probe={record['baseline_probe']}->{record['mutated_probe']} "
                    f"baseline_json={record['baseline_diagnostic_json']} "
                    f"mutated_json={record['mutated_diagnostic_json']}",
                    flush=True,
                )
                if record["baseline_probe"] != "R0001" or record["mutated_probe"] != "R0002":
                    record["status"] = "survived"
                    record["debt_probe_mismatch"] = True
            records.append(record)
            print(f"{mid} {record['status']}", flush=True)
            if record["status"] not in ("killed", "debt") or sha(original) != sha(restored):
                record["evidence_tail"] = output[-4000:]
                break
        if not requested and len(records) == len(MUTATIONS):
            for item in EQUIVALENTS:
                mid, file, basis, *probe = item
                path = COPY / file
                original = path.read_bytes()
                equivalent_record = {
                    "id": mid,
                    "status": "equivalent",
                    "equivalence_basis": basis,
                    "production_sha256": sha(original),
                    "restored_sha256": sha(original),
                }
                if probe:
                    old, new, occurrence, target, test = probe[0]
                    before, mutated = mutate(COPY, file, old, new, occurrence)
                    killed, output = run_test(COPY, target, test)
                    path.write_bytes(before)
                    restored = path.read_bytes()
                    equivalent_record.update(
                        {
                            "status": "killed" if killed else "equivalent",
                            "test": f"{target}::{test}",
                            "mutated_sha256": sha(mutated),
                            "production_sha256": sha(before),
                            "restored_sha256": sha(restored),
                        }
                    )
                    if not killed and ("error: could not compile" in output or "error[E" in output):
                        equivalent_record["status"] = "compile_error"
                        equivalent_record["evidence_tail"] = output[-4000:]
                records.append(equivalent_record)
                print(f"{mid} {equivalent_record['status']}", flush=True)
        if not requested:
            payload = json.dumps(
                {
                    "cut": "I11a",
                "population_source": "working_tree",
                "population_profile": "debug",
                "observability_dimensions": MUTATION_DIMENSIONS,
                "mutation_axes": MUTATION_AXES,
                "debt_mutations": sorted(DEBT_MUTATIONS),
                "observation_debts": OBSERVATION_DEBTS,
                "mutations": records,
                },
                ensure_ascii=False,
                indent=2,
                sort_keys=True,
            ) + "\n"
            OUTPUT.write_bytes(payload.encode("utf-8"))
        expected = len(selected) + (len(EQUIVALENTS) if not requested else 0)
        allowed_statuses = ("killed", "equivalent", "debt")
        return 0 if len(records) == expected and all(item["status"] in allowed_statuses and item["production_sha256"] == item["restored_sha256"] for item in records) else 1
    finally:
        shutil.rmtree(COPY, ignore_errors=True)


if __name__ == "__main__":
    raise SystemExit(main())
