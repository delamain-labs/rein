#include "tree_sitter/parser.h"

#if defined(__GNUC__) || defined(__clang__)
#pragma GCC diagnostic ignored "-Wmissing-field-initializers"
#endif

#define LANGUAGE_VERSION 14
#define STATE_COUNT 141
#define LARGE_STATE_COUNT 2
#define SYMBOL_COUNT 95
#define ALIAS_COUNT 0
#define TOKEN_COUNT 46
#define EXTERNAL_TOKEN_COUNT 0
#define FIELD_COUNT 1
#define MAX_ALIAS_SEQUENCE_LENGTH 6
#define PRODUCTION_ID_COUNT 2

enum ts_symbol_identifiers {
  sym_identifier = 1,
  sym_comment = 2,
  anon_sym_import = 3,
  anon_sym_STAR = 4,
  anon_sym_from = 5,
  anon_sym_defaults = 6,
  anon_sym_provider = 7,
  anon_sym_tool = 8,
  anon_sym_archetype = 9,
  anon_sym_type = 10,
  anon_sym_policy = 11,
  anon_sym_agent = 12,
  anon_sym_LBRACE = 13,
  anon_sym_RBRACE = 14,
  anon_sym_model = 15,
  anon_sym_COLON = 16,
  anon_sym_can = 17,
  anon_sym_LBRACK = 18,
  anon_sym_RBRACK = 19,
  anon_sym_cannot = 20,
  anon_sym_up = 21,
  anon_sym_to = 22,
  anon_sym_budget = 23,
  anon_sym_per = 24,
  anon_sym_guardrails = 25,
  anon_sym_workflow = 26,
  anon_sym_trigger = 27,
  anon_sym_stages = 28,
  anon_sym_COMMA = 29,
  anon_sym_step = 30,
  anon_sym_route = 31,
  anon_sym_on = 32,
  anon_sym_DASH_GT = 33,
  anon_sym_parallel = 34,
  anon_sym_observe = 35,
  anon_sym_fleet = 36,
  anon_sym_channel = 37,
  anon_sym_circuit_breaker = 38,
  anon_sym_DOT = 39,
  sym_currency = 40,
  sym_number = 41,
  anon_sym_DQUOTE = 42,
  aux_sym_string_token1 = 43,
  anon_sym_true = 44,
  anon_sym_false = 45,
  sym_source_file = 46,
  sym__definition = 47,
  sym_import_def = 48,
  sym_defaults_def = 49,
  sym_provider_def = 50,
  sym_tool_def = 51,
  sym_archetype_def = 52,
  sym_type_def = 53,
  sym_policy_def = 54,
  sym_agent_def = 55,
  sym__agent_field = 56,
  sym_model_field = 57,
  sym_can_block = 58,
  sym_cannot_block = 59,
  sym_capability = 60,
  sym_capability_constraint = 61,
  sym_budget_field = 62,
  sym_guardrails_field = 63,
  sym_workflow_def = 64,
  sym__workflow_field = 65,
  sym_trigger_field = 66,
  sym_stages_field = 67,
  sym_step_def = 68,
  sym_route_block = 69,
  sym_route_arm = 70,
  sym_parallel_block = 71,
  sym_observe_def = 72,
  sym_fleet_def = 73,
  sym_channel_def = 74,
  sym_circuit_breaker_def = 75,
  sym_block = 76,
  sym__block_item = 77,
  sym_key_value = 78,
  sym_list_field = 79,
  sym_block_field = 80,
  sym__value = 81,
  sym_dotted_name = 82,
  sym_string = 83,
  sym_boolean = 84,
  aux_sym_source_file_repeat1 = 85,
  aux_sym_agent_def_repeat1 = 86,
  aux_sym_can_block_repeat1 = 87,
  aux_sym_guardrails_field_repeat1 = 88,
  aux_sym_workflow_def_repeat1 = 89,
  aux_sym_stages_field_repeat1 = 90,
  aux_sym_route_block_repeat1 = 91,
  aux_sym_block_repeat1 = 92,
  aux_sym_list_field_repeat1 = 93,
  aux_sym_dotted_name_repeat1 = 94,
};

static const char * const ts_symbol_names[] = {
  [ts_builtin_sym_end] = "end",
  [sym_identifier] = "identifier",
  [sym_comment] = "comment",
  [anon_sym_import] = "import",
  [anon_sym_STAR] = "*",
  [anon_sym_from] = "from",
  [anon_sym_defaults] = "defaults",
  [anon_sym_provider] = "provider",
  [anon_sym_tool] = "tool",
  [anon_sym_archetype] = "archetype",
  [anon_sym_type] = "type",
  [anon_sym_policy] = "policy",
  [anon_sym_agent] = "agent",
  [anon_sym_LBRACE] = "{",
  [anon_sym_RBRACE] = "}",
  [anon_sym_model] = "model",
  [anon_sym_COLON] = ":",
  [anon_sym_can] = "can",
  [anon_sym_LBRACK] = "[",
  [anon_sym_RBRACK] = "]",
  [anon_sym_cannot] = "cannot",
  [anon_sym_up] = "up",
  [anon_sym_to] = "to",
  [anon_sym_budget] = "budget",
  [anon_sym_per] = "per",
  [anon_sym_guardrails] = "guardrails",
  [anon_sym_workflow] = "workflow",
  [anon_sym_trigger] = "trigger",
  [anon_sym_stages] = "stages",
  [anon_sym_COMMA] = ",",
  [anon_sym_step] = "step",
  [anon_sym_route] = "route",
  [anon_sym_on] = "on",
  [anon_sym_DASH_GT] = "->",
  [anon_sym_parallel] = "parallel",
  [anon_sym_observe] = "observe",
  [anon_sym_fleet] = "fleet",
  [anon_sym_channel] = "channel",
  [anon_sym_circuit_breaker] = "circuit_breaker",
  [anon_sym_DOT] = ".",
  [sym_currency] = "currency",
  [sym_number] = "number",
  [anon_sym_DQUOTE] = "\"",
  [aux_sym_string_token1] = "string_token1",
  [anon_sym_true] = "true",
  [anon_sym_false] = "false",
  [sym_source_file] = "source_file",
  [sym__definition] = "_definition",
  [sym_import_def] = "import_def",
  [sym_defaults_def] = "defaults_def",
  [sym_provider_def] = "provider_def",
  [sym_tool_def] = "tool_def",
  [sym_archetype_def] = "archetype_def",
  [sym_type_def] = "type_def",
  [sym_policy_def] = "policy_def",
  [sym_agent_def] = "agent_def",
  [sym__agent_field] = "_agent_field",
  [sym_model_field] = "model_field",
  [sym_can_block] = "can_block",
  [sym_cannot_block] = "cannot_block",
  [sym_capability] = "capability",
  [sym_capability_constraint] = "capability_constraint",
  [sym_budget_field] = "budget_field",
  [sym_guardrails_field] = "guardrails_field",
  [sym_workflow_def] = "workflow_def",
  [sym__workflow_field] = "_workflow_field",
  [sym_trigger_field] = "trigger_field",
  [sym_stages_field] = "stages_field",
  [sym_step_def] = "step_def",
  [sym_route_block] = "route_block",
  [sym_route_arm] = "route_arm",
  [sym_parallel_block] = "parallel_block",
  [sym_observe_def] = "observe_def",
  [sym_fleet_def] = "fleet_def",
  [sym_channel_def] = "channel_def",
  [sym_circuit_breaker_def] = "circuit_breaker_def",
  [sym_block] = "block",
  [sym__block_item] = "_block_item",
  [sym_key_value] = "key_value",
  [sym_list_field] = "list_field",
  [sym_block_field] = "block_field",
  [sym__value] = "_value",
  [sym_dotted_name] = "dotted_name",
  [sym_string] = "string",
  [sym_boolean] = "boolean",
  [aux_sym_source_file_repeat1] = "source_file_repeat1",
  [aux_sym_agent_def_repeat1] = "agent_def_repeat1",
  [aux_sym_can_block_repeat1] = "can_block_repeat1",
  [aux_sym_guardrails_field_repeat1] = "guardrails_field_repeat1",
  [aux_sym_workflow_def_repeat1] = "workflow_def_repeat1",
  [aux_sym_stages_field_repeat1] = "stages_field_repeat1",
  [aux_sym_route_block_repeat1] = "route_block_repeat1",
  [aux_sym_block_repeat1] = "block_repeat1",
  [aux_sym_list_field_repeat1] = "list_field_repeat1",
  [aux_sym_dotted_name_repeat1] = "dotted_name_repeat1",
};

static const TSSymbol ts_symbol_map[] = {
  [ts_builtin_sym_end] = ts_builtin_sym_end,
  [sym_identifier] = sym_identifier,
  [sym_comment] = sym_comment,
  [anon_sym_import] = anon_sym_import,
  [anon_sym_STAR] = anon_sym_STAR,
  [anon_sym_from] = anon_sym_from,
  [anon_sym_defaults] = anon_sym_defaults,
  [anon_sym_provider] = anon_sym_provider,
  [anon_sym_tool] = anon_sym_tool,
  [anon_sym_archetype] = anon_sym_archetype,
  [anon_sym_type] = anon_sym_type,
  [anon_sym_policy] = anon_sym_policy,
  [anon_sym_agent] = anon_sym_agent,
  [anon_sym_LBRACE] = anon_sym_LBRACE,
  [anon_sym_RBRACE] = anon_sym_RBRACE,
  [anon_sym_model] = anon_sym_model,
  [anon_sym_COLON] = anon_sym_COLON,
  [anon_sym_can] = anon_sym_can,
  [anon_sym_LBRACK] = anon_sym_LBRACK,
  [anon_sym_RBRACK] = anon_sym_RBRACK,
  [anon_sym_cannot] = anon_sym_cannot,
  [anon_sym_up] = anon_sym_up,
  [anon_sym_to] = anon_sym_to,
  [anon_sym_budget] = anon_sym_budget,
  [anon_sym_per] = anon_sym_per,
  [anon_sym_guardrails] = anon_sym_guardrails,
  [anon_sym_workflow] = anon_sym_workflow,
  [anon_sym_trigger] = anon_sym_trigger,
  [anon_sym_stages] = anon_sym_stages,
  [anon_sym_COMMA] = anon_sym_COMMA,
  [anon_sym_step] = anon_sym_step,
  [anon_sym_route] = anon_sym_route,
  [anon_sym_on] = anon_sym_on,
  [anon_sym_DASH_GT] = anon_sym_DASH_GT,
  [anon_sym_parallel] = anon_sym_parallel,
  [anon_sym_observe] = anon_sym_observe,
  [anon_sym_fleet] = anon_sym_fleet,
  [anon_sym_channel] = anon_sym_channel,
  [anon_sym_circuit_breaker] = anon_sym_circuit_breaker,
  [anon_sym_DOT] = anon_sym_DOT,
  [sym_currency] = sym_currency,
  [sym_number] = sym_number,
  [anon_sym_DQUOTE] = anon_sym_DQUOTE,
  [aux_sym_string_token1] = aux_sym_string_token1,
  [anon_sym_true] = anon_sym_true,
  [anon_sym_false] = anon_sym_false,
  [sym_source_file] = sym_source_file,
  [sym__definition] = sym__definition,
  [sym_import_def] = sym_import_def,
  [sym_defaults_def] = sym_defaults_def,
  [sym_provider_def] = sym_provider_def,
  [sym_tool_def] = sym_tool_def,
  [sym_archetype_def] = sym_archetype_def,
  [sym_type_def] = sym_type_def,
  [sym_policy_def] = sym_policy_def,
  [sym_agent_def] = sym_agent_def,
  [sym__agent_field] = sym__agent_field,
  [sym_model_field] = sym_model_field,
  [sym_can_block] = sym_can_block,
  [sym_cannot_block] = sym_cannot_block,
  [sym_capability] = sym_capability,
  [sym_capability_constraint] = sym_capability_constraint,
  [sym_budget_field] = sym_budget_field,
  [sym_guardrails_field] = sym_guardrails_field,
  [sym_workflow_def] = sym_workflow_def,
  [sym__workflow_field] = sym__workflow_field,
  [sym_trigger_field] = sym_trigger_field,
  [sym_stages_field] = sym_stages_field,
  [sym_step_def] = sym_step_def,
  [sym_route_block] = sym_route_block,
  [sym_route_arm] = sym_route_arm,
  [sym_parallel_block] = sym_parallel_block,
  [sym_observe_def] = sym_observe_def,
  [sym_fleet_def] = sym_fleet_def,
  [sym_channel_def] = sym_channel_def,
  [sym_circuit_breaker_def] = sym_circuit_breaker_def,
  [sym_block] = sym_block,
  [sym__block_item] = sym__block_item,
  [sym_key_value] = sym_key_value,
  [sym_list_field] = sym_list_field,
  [sym_block_field] = sym_block_field,
  [sym__value] = sym__value,
  [sym_dotted_name] = sym_dotted_name,
  [sym_string] = sym_string,
  [sym_boolean] = sym_boolean,
  [aux_sym_source_file_repeat1] = aux_sym_source_file_repeat1,
  [aux_sym_agent_def_repeat1] = aux_sym_agent_def_repeat1,
  [aux_sym_can_block_repeat1] = aux_sym_can_block_repeat1,
  [aux_sym_guardrails_field_repeat1] = aux_sym_guardrails_field_repeat1,
  [aux_sym_workflow_def_repeat1] = aux_sym_workflow_def_repeat1,
  [aux_sym_stages_field_repeat1] = aux_sym_stages_field_repeat1,
  [aux_sym_route_block_repeat1] = aux_sym_route_block_repeat1,
  [aux_sym_block_repeat1] = aux_sym_block_repeat1,
  [aux_sym_list_field_repeat1] = aux_sym_list_field_repeat1,
  [aux_sym_dotted_name_repeat1] = aux_sym_dotted_name_repeat1,
};

static const TSSymbolMetadata ts_symbol_metadata[] = {
  [ts_builtin_sym_end] = {
    .visible = false,
    .named = true,
  },
  [sym_identifier] = {
    .visible = true,
    .named = true,
  },
  [sym_comment] = {
    .visible = true,
    .named = true,
  },
  [anon_sym_import] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_STAR] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_from] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_defaults] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_provider] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_tool] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_archetype] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_type] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_policy] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_agent] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_LBRACE] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_RBRACE] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_model] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_COLON] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_can] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_LBRACK] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_RBRACK] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_cannot] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_up] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_to] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_budget] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_per] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_guardrails] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_workflow] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_trigger] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_stages] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_COMMA] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_step] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_route] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_on] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_DASH_GT] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_parallel] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_observe] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_fleet] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_channel] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_circuit_breaker] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_DOT] = {
    .visible = true,
    .named = false,
  },
  [sym_currency] = {
    .visible = true,
    .named = true,
  },
  [sym_number] = {
    .visible = true,
    .named = true,
  },
  [anon_sym_DQUOTE] = {
    .visible = true,
    .named = false,
  },
  [aux_sym_string_token1] = {
    .visible = false,
    .named = false,
  },
  [anon_sym_true] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_false] = {
    .visible = true,
    .named = false,
  },
  [sym_source_file] = {
    .visible = true,
    .named = true,
  },
  [sym__definition] = {
    .visible = false,
    .named = true,
  },
  [sym_import_def] = {
    .visible = true,
    .named = true,
  },
  [sym_defaults_def] = {
    .visible = true,
    .named = true,
  },
  [sym_provider_def] = {
    .visible = true,
    .named = true,
  },
  [sym_tool_def] = {
    .visible = true,
    .named = true,
  },
  [sym_archetype_def] = {
    .visible = true,
    .named = true,
  },
  [sym_type_def] = {
    .visible = true,
    .named = true,
  },
  [sym_policy_def] = {
    .visible = true,
    .named = true,
  },
  [sym_agent_def] = {
    .visible = true,
    .named = true,
  },
  [sym__agent_field] = {
    .visible = false,
    .named = true,
  },
  [sym_model_field] = {
    .visible = true,
    .named = true,
  },
  [sym_can_block] = {
    .visible = true,
    .named = true,
  },
  [sym_cannot_block] = {
    .visible = true,
    .named = true,
  },
  [sym_capability] = {
    .visible = true,
    .named = true,
  },
  [sym_capability_constraint] = {
    .visible = true,
    .named = true,
  },
  [sym_budget_field] = {
    .visible = true,
    .named = true,
  },
  [sym_guardrails_field] = {
    .visible = true,
    .named = true,
  },
  [sym_workflow_def] = {
    .visible = true,
    .named = true,
  },
  [sym__workflow_field] = {
    .visible = false,
    .named = true,
  },
  [sym_trigger_field] = {
    .visible = true,
    .named = true,
  },
  [sym_stages_field] = {
    .visible = true,
    .named = true,
  },
  [sym_step_def] = {
    .visible = true,
    .named = true,
  },
  [sym_route_block] = {
    .visible = true,
    .named = true,
  },
  [sym_route_arm] = {
    .visible = true,
    .named = true,
  },
  [sym_parallel_block] = {
    .visible = true,
    .named = true,
  },
  [sym_observe_def] = {
    .visible = true,
    .named = true,
  },
  [sym_fleet_def] = {
    .visible = true,
    .named = true,
  },
  [sym_channel_def] = {
    .visible = true,
    .named = true,
  },
  [sym_circuit_breaker_def] = {
    .visible = true,
    .named = true,
  },
  [sym_block] = {
    .visible = true,
    .named = true,
  },
  [sym__block_item] = {
    .visible = false,
    .named = true,
  },
  [sym_key_value] = {
    .visible = true,
    .named = true,
  },
  [sym_list_field] = {
    .visible = true,
    .named = true,
  },
  [sym_block_field] = {
    .visible = true,
    .named = true,
  },
  [sym__value] = {
    .visible = false,
    .named = true,
  },
  [sym_dotted_name] = {
    .visible = true,
    .named = true,
  },
  [sym_string] = {
    .visible = true,
    .named = true,
  },
  [sym_boolean] = {
    .visible = true,
    .named = true,
  },
  [aux_sym_source_file_repeat1] = {
    .visible = false,
    .named = false,
  },
  [aux_sym_agent_def_repeat1] = {
    .visible = false,
    .named = false,
  },
  [aux_sym_can_block_repeat1] = {
    .visible = false,
    .named = false,
  },
  [aux_sym_guardrails_field_repeat1] = {
    .visible = false,
    .named = false,
  },
  [aux_sym_workflow_def_repeat1] = {
    .visible = false,
    .named = false,
  },
  [aux_sym_stages_field_repeat1] = {
    .visible = false,
    .named = false,
  },
  [aux_sym_route_block_repeat1] = {
    .visible = false,
    .named = false,
  },
  [aux_sym_block_repeat1] = {
    .visible = false,
    .named = false,
  },
  [aux_sym_list_field_repeat1] = {
    .visible = false,
    .named = false,
  },
  [aux_sym_dotted_name_repeat1] = {
    .visible = false,
    .named = false,
  },
};

enum ts_field_identifiers {
  field_name = 1,
};

static const char * const ts_field_names[] = {
  [0] = NULL,
  [field_name] = "name",
};

static const TSFieldMapSlice ts_field_map_slices[PRODUCTION_ID_COUNT] = {
  [1] = {.index = 0, .length = 1},
};

static const TSFieldMapEntry ts_field_map_entries[] = {
  [0] =
    {field_name, 1},
};

static const TSSymbol ts_alias_sequences[PRODUCTION_ID_COUNT][MAX_ALIAS_SEQUENCE_LENGTH] = {
  [0] = {0},
};

static const uint16_t ts_non_terminal_alias_map[] = {
  0,
};

static const TSStateId ts_primary_state_ids[STATE_COUNT] = {
  [0] = 0,
  [1] = 1,
  [2] = 2,
  [3] = 3,
  [4] = 4,
  [5] = 5,
  [6] = 6,
  [7] = 7,
  [8] = 8,
  [9] = 9,
  [10] = 10,
  [11] = 11,
  [12] = 12,
  [13] = 13,
  [14] = 14,
  [15] = 15,
  [16] = 16,
  [17] = 17,
  [18] = 18,
  [19] = 19,
  [20] = 20,
  [21] = 21,
  [22] = 22,
  [23] = 23,
  [24] = 24,
  [25] = 25,
  [26] = 26,
  [27] = 27,
  [28] = 28,
  [29] = 29,
  [30] = 30,
  [31] = 31,
  [32] = 32,
  [33] = 33,
  [34] = 34,
  [35] = 35,
  [36] = 36,
  [37] = 37,
  [38] = 38,
  [39] = 39,
  [40] = 40,
  [41] = 41,
  [42] = 42,
  [43] = 43,
  [44] = 44,
  [45] = 45,
  [46] = 46,
  [47] = 47,
  [48] = 48,
  [49] = 49,
  [50] = 50,
  [51] = 51,
  [52] = 52,
  [53] = 53,
  [54] = 54,
  [55] = 55,
  [56] = 56,
  [57] = 57,
  [58] = 58,
  [59] = 59,
  [60] = 60,
  [61] = 61,
  [62] = 62,
  [63] = 63,
  [64] = 64,
  [65] = 65,
  [66] = 66,
  [67] = 67,
  [68] = 68,
  [69] = 69,
  [70] = 70,
  [71] = 71,
  [72] = 72,
  [73] = 73,
  [74] = 74,
  [75] = 75,
  [76] = 76,
  [77] = 77,
  [78] = 78,
  [79] = 79,
  [80] = 80,
  [81] = 81,
  [82] = 82,
  [83] = 83,
  [84] = 84,
  [85] = 85,
  [86] = 86,
  [87] = 87,
  [88] = 88,
  [89] = 89,
  [90] = 90,
  [91] = 91,
  [92] = 92,
  [93] = 93,
  [94] = 94,
  [95] = 95,
  [96] = 96,
  [97] = 97,
  [98] = 98,
  [99] = 99,
  [100] = 100,
  [101] = 101,
  [102] = 102,
  [103] = 103,
  [104] = 104,
  [105] = 105,
  [106] = 106,
  [107] = 107,
  [108] = 108,
  [109] = 109,
  [110] = 110,
  [111] = 111,
  [112] = 112,
  [113] = 113,
  [114] = 114,
  [115] = 115,
  [116] = 116,
  [117] = 117,
  [118] = 118,
  [119] = 119,
  [120] = 120,
  [121] = 121,
  [122] = 122,
  [123] = 123,
  [124] = 124,
  [125] = 125,
  [126] = 126,
  [127] = 127,
  [128] = 128,
  [129] = 129,
  [130] = 130,
  [131] = 131,
  [132] = 132,
  [133] = 133,
  [134] = 134,
  [135] = 135,
  [136] = 136,
  [137] = 137,
  [138] = 138,
  [139] = 139,
  [140] = 140,
};

static bool ts_lex(TSLexer *lexer, TSStateId state) {
  START_LEXER();
  eof = lexer->eof(lexer);
  switch (state) {
    case 0:
      if (eof) ADVANCE(6);
      ADVANCE_MAP(
        '"', 23,
        '$', 3,
        '*', 9,
        ',', 15,
        '-', 2,
        '.', 17,
        '/', 1,
        ':', 12,
        '[', 13,
        ']', 14,
        '{', 10,
        '}', 11,
      );
      if (('\t' <= lookahead && lookahead <= '\r') ||
          lookahead == ' ') SKIP(0);
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(21);
      if (('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(27);
      END_STATE();
    case 1:
      if (lookahead == '/') ADVANCE(8);
      END_STATE();
    case 2:
      if (lookahead == '>') ADVANCE(16);
      END_STATE();
    case 3:
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(19);
      END_STATE();
    case 4:
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(22);
      END_STATE();
    case 5:
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(20);
      END_STATE();
    case 6:
      ACCEPT_TOKEN(ts_builtin_sym_end);
      END_STATE();
    case 7:
      ACCEPT_TOKEN(sym_comment);
      if (lookahead == '\n') ADVANCE(26);
      if (lookahead == '"') ADVANCE(8);
      if (lookahead != 0) ADVANCE(7);
      END_STATE();
    case 8:
      ACCEPT_TOKEN(sym_comment);
      if (lookahead != 0 &&
          lookahead != '\n') ADVANCE(8);
      END_STATE();
    case 9:
      ACCEPT_TOKEN(anon_sym_STAR);
      END_STATE();
    case 10:
      ACCEPT_TOKEN(anon_sym_LBRACE);
      END_STATE();
    case 11:
      ACCEPT_TOKEN(anon_sym_RBRACE);
      END_STATE();
    case 12:
      ACCEPT_TOKEN(anon_sym_COLON);
      END_STATE();
    case 13:
      ACCEPT_TOKEN(anon_sym_LBRACK);
      END_STATE();
    case 14:
      ACCEPT_TOKEN(anon_sym_RBRACK);
      END_STATE();
    case 15:
      ACCEPT_TOKEN(anon_sym_COMMA);
      END_STATE();
    case 16:
      ACCEPT_TOKEN(anon_sym_DASH_GT);
      END_STATE();
    case 17:
      ACCEPT_TOKEN(anon_sym_DOT);
      END_STATE();
    case 18:
      ACCEPT_TOKEN(sym_currency);
      END_STATE();
    case 19:
      ACCEPT_TOKEN(sym_currency);
      if (lookahead == '.') ADVANCE(5);
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(19);
      END_STATE();
    case 20:
      ACCEPT_TOKEN(sym_currency);
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(18);
      END_STATE();
    case 21:
      ACCEPT_TOKEN(sym_number);
      if (lookahead == '.') ADVANCE(4);
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(21);
      END_STATE();
    case 22:
      ACCEPT_TOKEN(sym_number);
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(22);
      END_STATE();
    case 23:
      ACCEPT_TOKEN(anon_sym_DQUOTE);
      END_STATE();
    case 24:
      ACCEPT_TOKEN(aux_sym_string_token1);
      if (lookahead == '/') ADVANCE(25);
      if (('\t' <= lookahead && lookahead <= '\r') ||
          lookahead == ' ') ADVANCE(24);
      if (lookahead != 0 &&
          lookahead != '"') ADVANCE(26);
      END_STATE();
    case 25:
      ACCEPT_TOKEN(aux_sym_string_token1);
      if (lookahead == '/') ADVANCE(7);
      if (lookahead != 0 &&
          lookahead != '"') ADVANCE(26);
      END_STATE();
    case 26:
      ACCEPT_TOKEN(aux_sym_string_token1);
      if (lookahead != 0 &&
          lookahead != '"') ADVANCE(26);
      END_STATE();
    case 27:
      ACCEPT_TOKEN(sym_identifier);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(27);
      END_STATE();
    default:
      return false;
  }
}

static bool ts_lex_keywords(TSLexer *lexer, TSStateId state) {
  START_LEXER();
  eof = lexer->eof(lexer);
  switch (state) {
    case 0:
      ADVANCE_MAP(
        'a', 1,
        'b', 2,
        'c', 3,
        'd', 4,
        'f', 5,
        'g', 6,
        'i', 7,
        'm', 8,
        'o', 9,
        'p', 10,
        'r', 11,
        's', 12,
        't', 13,
        'u', 14,
        'w', 15,
      );
      if (('\t' <= lookahead && lookahead <= '\r') ||
          lookahead == ' ') SKIP(0);
      END_STATE();
    case 1:
      if (lookahead == 'g') ADVANCE(16);
      if (lookahead == 'r') ADVANCE(17);
      END_STATE();
    case 2:
      if (lookahead == 'u') ADVANCE(18);
      END_STATE();
    case 3:
      if (lookahead == 'a') ADVANCE(19);
      if (lookahead == 'h') ADVANCE(20);
      if (lookahead == 'i') ADVANCE(21);
      END_STATE();
    case 4:
      if (lookahead == 'e') ADVANCE(22);
      END_STATE();
    case 5:
      if (lookahead == 'a') ADVANCE(23);
      if (lookahead == 'l') ADVANCE(24);
      if (lookahead == 'r') ADVANCE(25);
      END_STATE();
    case 6:
      if (lookahead == 'u') ADVANCE(26);
      END_STATE();
    case 7:
      if (lookahead == 'm') ADVANCE(27);
      END_STATE();
    case 8:
      if (lookahead == 'o') ADVANCE(28);
      END_STATE();
    case 9:
      if (lookahead == 'b') ADVANCE(29);
      if (lookahead == 'n') ADVANCE(30);
      END_STATE();
    case 10:
      if (lookahead == 'a') ADVANCE(31);
      if (lookahead == 'e') ADVANCE(32);
      if (lookahead == 'o') ADVANCE(33);
      if (lookahead == 'r') ADVANCE(34);
      END_STATE();
    case 11:
      if (lookahead == 'o') ADVANCE(35);
      END_STATE();
    case 12:
      if (lookahead == 't') ADVANCE(36);
      END_STATE();
    case 13:
      if (lookahead == 'o') ADVANCE(37);
      if (lookahead == 'r') ADVANCE(38);
      if (lookahead == 'y') ADVANCE(39);
      END_STATE();
    case 14:
      if (lookahead == 'p') ADVANCE(40);
      END_STATE();
    case 15:
      if (lookahead == 'o') ADVANCE(41);
      END_STATE();
    case 16:
      if (lookahead == 'e') ADVANCE(42);
      END_STATE();
    case 17:
      if (lookahead == 'c') ADVANCE(43);
      END_STATE();
    case 18:
      if (lookahead == 'd') ADVANCE(44);
      END_STATE();
    case 19:
      if (lookahead == 'n') ADVANCE(45);
      END_STATE();
    case 20:
      if (lookahead == 'a') ADVANCE(46);
      END_STATE();
    case 21:
      if (lookahead == 'r') ADVANCE(47);
      END_STATE();
    case 22:
      if (lookahead == 'f') ADVANCE(48);
      END_STATE();
    case 23:
      if (lookahead == 'l') ADVANCE(49);
      END_STATE();
    case 24:
      if (lookahead == 'e') ADVANCE(50);
      END_STATE();
    case 25:
      if (lookahead == 'o') ADVANCE(51);
      END_STATE();
    case 26:
      if (lookahead == 'a') ADVANCE(52);
      END_STATE();
    case 27:
      if (lookahead == 'p') ADVANCE(53);
      END_STATE();
    case 28:
      if (lookahead == 'd') ADVANCE(54);
      END_STATE();
    case 29:
      if (lookahead == 's') ADVANCE(55);
      END_STATE();
    case 30:
      ACCEPT_TOKEN(anon_sym_on);
      END_STATE();
    case 31:
      if (lookahead == 'r') ADVANCE(56);
      END_STATE();
    case 32:
      if (lookahead == 'r') ADVANCE(57);
      END_STATE();
    case 33:
      if (lookahead == 'l') ADVANCE(58);
      END_STATE();
    case 34:
      if (lookahead == 'o') ADVANCE(59);
      END_STATE();
    case 35:
      if (lookahead == 'u') ADVANCE(60);
      END_STATE();
    case 36:
      if (lookahead == 'a') ADVANCE(61);
      if (lookahead == 'e') ADVANCE(62);
      END_STATE();
    case 37:
      ACCEPT_TOKEN(anon_sym_to);
      if (lookahead == 'o') ADVANCE(63);
      END_STATE();
    case 38:
      if (lookahead == 'i') ADVANCE(64);
      if (lookahead == 'u') ADVANCE(65);
      END_STATE();
    case 39:
      if (lookahead == 'p') ADVANCE(66);
      END_STATE();
    case 40:
      ACCEPT_TOKEN(anon_sym_up);
      END_STATE();
    case 41:
      if (lookahead == 'r') ADVANCE(67);
      END_STATE();
    case 42:
      if (lookahead == 'n') ADVANCE(68);
      END_STATE();
    case 43:
      if (lookahead == 'h') ADVANCE(69);
      END_STATE();
    case 44:
      if (lookahead == 'g') ADVANCE(70);
      END_STATE();
    case 45:
      ACCEPT_TOKEN(anon_sym_can);
      if (lookahead == 'n') ADVANCE(71);
      END_STATE();
    case 46:
      if (lookahead == 'n') ADVANCE(72);
      END_STATE();
    case 47:
      if (lookahead == 'c') ADVANCE(73);
      END_STATE();
    case 48:
      if (lookahead == 'a') ADVANCE(74);
      END_STATE();
    case 49:
      if (lookahead == 's') ADVANCE(75);
      END_STATE();
    case 50:
      if (lookahead == 'e') ADVANCE(76);
      END_STATE();
    case 51:
      if (lookahead == 'm') ADVANCE(77);
      END_STATE();
    case 52:
      if (lookahead == 'r') ADVANCE(78);
      END_STATE();
    case 53:
      if (lookahead == 'o') ADVANCE(79);
      END_STATE();
    case 54:
      if (lookahead == 'e') ADVANCE(80);
      END_STATE();
    case 55:
      if (lookahead == 'e') ADVANCE(81);
      END_STATE();
    case 56:
      if (lookahead == 'a') ADVANCE(82);
      END_STATE();
    case 57:
      ACCEPT_TOKEN(anon_sym_per);
      END_STATE();
    case 58:
      if (lookahead == 'i') ADVANCE(83);
      END_STATE();
    case 59:
      if (lookahead == 'v') ADVANCE(84);
      END_STATE();
    case 60:
      if (lookahead == 't') ADVANCE(85);
      END_STATE();
    case 61:
      if (lookahead == 'g') ADVANCE(86);
      END_STATE();
    case 62:
      if (lookahead == 'p') ADVANCE(87);
      END_STATE();
    case 63:
      if (lookahead == 'l') ADVANCE(88);
      END_STATE();
    case 64:
      if (lookahead == 'g') ADVANCE(89);
      END_STATE();
    case 65:
      if (lookahead == 'e') ADVANCE(90);
      END_STATE();
    case 66:
      if (lookahead == 'e') ADVANCE(91);
      END_STATE();
    case 67:
      if (lookahead == 'k') ADVANCE(92);
      END_STATE();
    case 68:
      if (lookahead == 't') ADVANCE(93);
      END_STATE();
    case 69:
      if (lookahead == 'e') ADVANCE(94);
      END_STATE();
    case 70:
      if (lookahead == 'e') ADVANCE(95);
      END_STATE();
    case 71:
      if (lookahead == 'o') ADVANCE(96);
      END_STATE();
    case 72:
      if (lookahead == 'n') ADVANCE(97);
      END_STATE();
    case 73:
      if (lookahead == 'u') ADVANCE(98);
      END_STATE();
    case 74:
      if (lookahead == 'u') ADVANCE(99);
      END_STATE();
    case 75:
      if (lookahead == 'e') ADVANCE(100);
      END_STATE();
    case 76:
      if (lookahead == 't') ADVANCE(101);
      END_STATE();
    case 77:
      ACCEPT_TOKEN(anon_sym_from);
      END_STATE();
    case 78:
      if (lookahead == 'd') ADVANCE(102);
      END_STATE();
    case 79:
      if (lookahead == 'r') ADVANCE(103);
      END_STATE();
    case 80:
      if (lookahead == 'l') ADVANCE(104);
      END_STATE();
    case 81:
      if (lookahead == 'r') ADVANCE(105);
      END_STATE();
    case 82:
      if (lookahead == 'l') ADVANCE(106);
      END_STATE();
    case 83:
      if (lookahead == 'c') ADVANCE(107);
      END_STATE();
    case 84:
      if (lookahead == 'i') ADVANCE(108);
      END_STATE();
    case 85:
      if (lookahead == 'e') ADVANCE(109);
      END_STATE();
    case 86:
      if (lookahead == 'e') ADVANCE(110);
      END_STATE();
    case 87:
      ACCEPT_TOKEN(anon_sym_step);
      END_STATE();
    case 88:
      ACCEPT_TOKEN(anon_sym_tool);
      END_STATE();
    case 89:
      if (lookahead == 'g') ADVANCE(111);
      END_STATE();
    case 90:
      ACCEPT_TOKEN(anon_sym_true);
      END_STATE();
    case 91:
      ACCEPT_TOKEN(anon_sym_type);
      END_STATE();
    case 92:
      if (lookahead == 'f') ADVANCE(112);
      END_STATE();
    case 93:
      ACCEPT_TOKEN(anon_sym_agent);
      END_STATE();
    case 94:
      if (lookahead == 't') ADVANCE(113);
      END_STATE();
    case 95:
      if (lookahead == 't') ADVANCE(114);
      END_STATE();
    case 96:
      if (lookahead == 't') ADVANCE(115);
      END_STATE();
    case 97:
      if (lookahead == 'e') ADVANCE(116);
      END_STATE();
    case 98:
      if (lookahead == 'i') ADVANCE(117);
      END_STATE();
    case 99:
      if (lookahead == 'l') ADVANCE(118);
      END_STATE();
    case 100:
      ACCEPT_TOKEN(anon_sym_false);
      END_STATE();
    case 101:
      ACCEPT_TOKEN(anon_sym_fleet);
      END_STATE();
    case 102:
      if (lookahead == 'r') ADVANCE(119);
      END_STATE();
    case 103:
      if (lookahead == 't') ADVANCE(120);
      END_STATE();
    case 104:
      ACCEPT_TOKEN(anon_sym_model);
      END_STATE();
    case 105:
      if (lookahead == 'v') ADVANCE(121);
      END_STATE();
    case 106:
      if (lookahead == 'l') ADVANCE(122);
      END_STATE();
    case 107:
      if (lookahead == 'y') ADVANCE(123);
      END_STATE();
    case 108:
      if (lookahead == 'd') ADVANCE(124);
      END_STATE();
    case 109:
      ACCEPT_TOKEN(anon_sym_route);
      END_STATE();
    case 110:
      if (lookahead == 's') ADVANCE(125);
      END_STATE();
    case 111:
      if (lookahead == 'e') ADVANCE(126);
      END_STATE();
    case 112:
      if (lookahead == 'l') ADVANCE(127);
      END_STATE();
    case 113:
      if (lookahead == 'y') ADVANCE(128);
      END_STATE();
    case 114:
      ACCEPT_TOKEN(anon_sym_budget);
      END_STATE();
    case 115:
      ACCEPT_TOKEN(anon_sym_cannot);
      END_STATE();
    case 116:
      if (lookahead == 'l') ADVANCE(129);
      END_STATE();
    case 117:
      if (lookahead == 't') ADVANCE(130);
      END_STATE();
    case 118:
      if (lookahead == 't') ADVANCE(131);
      END_STATE();
    case 119:
      if (lookahead == 'a') ADVANCE(132);
      END_STATE();
    case 120:
      ACCEPT_TOKEN(anon_sym_import);
      END_STATE();
    case 121:
      if (lookahead == 'e') ADVANCE(133);
      END_STATE();
    case 122:
      if (lookahead == 'e') ADVANCE(134);
      END_STATE();
    case 123:
      ACCEPT_TOKEN(anon_sym_policy);
      END_STATE();
    case 124:
      if (lookahead == 'e') ADVANCE(135);
      END_STATE();
    case 125:
      ACCEPT_TOKEN(anon_sym_stages);
      END_STATE();
    case 126:
      if (lookahead == 'r') ADVANCE(136);
      END_STATE();
    case 127:
      if (lookahead == 'o') ADVANCE(137);
      END_STATE();
    case 128:
      if (lookahead == 'p') ADVANCE(138);
      END_STATE();
    case 129:
      ACCEPT_TOKEN(anon_sym_channel);
      END_STATE();
    case 130:
      if (lookahead == '_') ADVANCE(139);
      END_STATE();
    case 131:
      if (lookahead == 's') ADVANCE(140);
      END_STATE();
    case 132:
      if (lookahead == 'i') ADVANCE(141);
      END_STATE();
    case 133:
      ACCEPT_TOKEN(anon_sym_observe);
      END_STATE();
    case 134:
      if (lookahead == 'l') ADVANCE(142);
      END_STATE();
    case 135:
      if (lookahead == 'r') ADVANCE(143);
      END_STATE();
    case 136:
      ACCEPT_TOKEN(anon_sym_trigger);
      END_STATE();
    case 137:
      if (lookahead == 'w') ADVANCE(144);
      END_STATE();
    case 138:
      if (lookahead == 'e') ADVANCE(145);
      END_STATE();
    case 139:
      if (lookahead == 'b') ADVANCE(146);
      END_STATE();
    case 140:
      ACCEPT_TOKEN(anon_sym_defaults);
      END_STATE();
    case 141:
      if (lookahead == 'l') ADVANCE(147);
      END_STATE();
    case 142:
      ACCEPT_TOKEN(anon_sym_parallel);
      END_STATE();
    case 143:
      ACCEPT_TOKEN(anon_sym_provider);
      END_STATE();
    case 144:
      ACCEPT_TOKEN(anon_sym_workflow);
      END_STATE();
    case 145:
      ACCEPT_TOKEN(anon_sym_archetype);
      END_STATE();
    case 146:
      if (lookahead == 'r') ADVANCE(148);
      END_STATE();
    case 147:
      if (lookahead == 's') ADVANCE(149);
      END_STATE();
    case 148:
      if (lookahead == 'e') ADVANCE(150);
      END_STATE();
    case 149:
      ACCEPT_TOKEN(anon_sym_guardrails);
      END_STATE();
    case 150:
      if (lookahead == 'a') ADVANCE(151);
      END_STATE();
    case 151:
      if (lookahead == 'k') ADVANCE(152);
      END_STATE();
    case 152:
      if (lookahead == 'e') ADVANCE(153);
      END_STATE();
    case 153:
      if (lookahead == 'r') ADVANCE(154);
      END_STATE();
    case 154:
      ACCEPT_TOKEN(anon_sym_circuit_breaker);
      END_STATE();
    default:
      return false;
  }
}

static const TSLexMode ts_lex_modes[STATE_COUNT] = {
  [0] = {.lex_state = 0},
  [1] = {.lex_state = 0},
  [2] = {.lex_state = 0},
  [3] = {.lex_state = 0},
  [4] = {.lex_state = 0},
  [5] = {.lex_state = 0},
  [6] = {.lex_state = 0},
  [7] = {.lex_state = 0},
  [8] = {.lex_state = 0},
  [9] = {.lex_state = 0},
  [10] = {.lex_state = 0},
  [11] = {.lex_state = 0},
  [12] = {.lex_state = 0},
  [13] = {.lex_state = 0},
  [14] = {.lex_state = 0},
  [15] = {.lex_state = 0},
  [16] = {.lex_state = 0},
  [17] = {.lex_state = 0},
  [18] = {.lex_state = 0},
  [19] = {.lex_state = 0},
  [20] = {.lex_state = 0},
  [21] = {.lex_state = 0},
  [22] = {.lex_state = 0},
  [23] = {.lex_state = 0},
  [24] = {.lex_state = 0},
  [25] = {.lex_state = 0},
  [26] = {.lex_state = 0},
  [27] = {.lex_state = 0},
  [28] = {.lex_state = 0},
  [29] = {.lex_state = 0},
  [30] = {.lex_state = 0},
  [31] = {.lex_state = 0},
  [32] = {.lex_state = 0},
  [33] = {.lex_state = 0},
  [34] = {.lex_state = 0},
  [35] = {.lex_state = 0},
  [36] = {.lex_state = 0},
  [37] = {.lex_state = 0},
  [38] = {.lex_state = 0},
  [39] = {.lex_state = 0},
  [40] = {.lex_state = 0},
  [41] = {.lex_state = 0},
  [42] = {.lex_state = 0},
  [43] = {.lex_state = 0},
  [44] = {.lex_state = 0},
  [45] = {.lex_state = 0},
  [46] = {.lex_state = 0},
  [47] = {.lex_state = 0},
  [48] = {.lex_state = 0},
  [49] = {.lex_state = 0},
  [50] = {.lex_state = 0},
  [51] = {.lex_state = 0},
  [52] = {.lex_state = 0},
  [53] = {.lex_state = 0},
  [54] = {.lex_state = 0},
  [55] = {.lex_state = 0},
  [56] = {.lex_state = 0},
  [57] = {.lex_state = 0},
  [58] = {.lex_state = 0},
  [59] = {.lex_state = 0},
  [60] = {.lex_state = 0},
  [61] = {.lex_state = 0},
  [62] = {.lex_state = 0},
  [63] = {.lex_state = 0},
  [64] = {.lex_state = 0},
  [65] = {.lex_state = 0},
  [66] = {.lex_state = 0},
  [67] = {.lex_state = 0},
  [68] = {.lex_state = 0},
  [69] = {.lex_state = 0},
  [70] = {.lex_state = 0},
  [71] = {.lex_state = 0},
  [72] = {.lex_state = 0},
  [73] = {.lex_state = 0},
  [74] = {.lex_state = 0},
  [75] = {.lex_state = 0},
  [76] = {.lex_state = 0},
  [77] = {.lex_state = 0},
  [78] = {.lex_state = 0},
  [79] = {.lex_state = 0},
  [80] = {.lex_state = 0},
  [81] = {.lex_state = 0},
  [82] = {.lex_state = 0},
  [83] = {.lex_state = 0},
  [84] = {.lex_state = 0},
  [85] = {.lex_state = 0},
  [86] = {.lex_state = 0},
  [87] = {.lex_state = 0},
  [88] = {.lex_state = 0},
  [89] = {.lex_state = 0},
  [90] = {.lex_state = 0},
  [91] = {.lex_state = 0},
  [92] = {.lex_state = 0},
  [93] = {.lex_state = 0},
  [94] = {.lex_state = 0},
  [95] = {.lex_state = 0},
  [96] = {.lex_state = 0},
  [97] = {.lex_state = 0},
  [98] = {.lex_state = 0},
  [99] = {.lex_state = 0},
  [100] = {.lex_state = 0},
  [101] = {.lex_state = 0},
  [102] = {.lex_state = 0},
  [103] = {.lex_state = 0},
  [104] = {.lex_state = 0},
  [105] = {.lex_state = 0},
  [106] = {.lex_state = 0},
  [107] = {.lex_state = 0},
  [108] = {.lex_state = 0},
  [109] = {.lex_state = 0},
  [110] = {.lex_state = 0},
  [111] = {.lex_state = 0},
  [112] = {.lex_state = 0},
  [113] = {.lex_state = 0},
  [114] = {.lex_state = 0},
  [115] = {.lex_state = 0},
  [116] = {.lex_state = 0},
  [117] = {.lex_state = 0},
  [118] = {.lex_state = 0},
  [119] = {.lex_state = 0},
  [120] = {.lex_state = 0},
  [121] = {.lex_state = 0},
  [122] = {.lex_state = 0},
  [123] = {.lex_state = 0},
  [124] = {.lex_state = 0},
  [125] = {.lex_state = 0},
  [126] = {.lex_state = 0},
  [127] = {.lex_state = 0},
  [128] = {.lex_state = 0},
  [129] = {.lex_state = 0},
  [130] = {.lex_state = 0},
  [131] = {.lex_state = 0},
  [132] = {.lex_state = 0},
  [133] = {.lex_state = 0},
  [134] = {.lex_state = 0},
  [135] = {.lex_state = 0},
  [136] = {.lex_state = 0},
  [137] = {.lex_state = 0},
  [138] = {.lex_state = 0},
  [139] = {.lex_state = 24},
  [140] = {.lex_state = 0},
};

static const uint16_t ts_parse_table[LARGE_STATE_COUNT][SYMBOL_COUNT] = {
  [0] = {
    [ts_builtin_sym_end] = ACTIONS(1),
    [sym_identifier] = ACTIONS(1),
    [sym_comment] = ACTIONS(3),
    [anon_sym_import] = ACTIONS(1),
    [anon_sym_STAR] = ACTIONS(1),
    [anon_sym_from] = ACTIONS(1),
    [anon_sym_defaults] = ACTIONS(1),
    [anon_sym_provider] = ACTIONS(1),
    [anon_sym_tool] = ACTIONS(1),
    [anon_sym_archetype] = ACTIONS(1),
    [anon_sym_type] = ACTIONS(1),
    [anon_sym_policy] = ACTIONS(1),
    [anon_sym_agent] = ACTIONS(1),
    [anon_sym_LBRACE] = ACTIONS(1),
    [anon_sym_RBRACE] = ACTIONS(1),
    [anon_sym_model] = ACTIONS(1),
    [anon_sym_COLON] = ACTIONS(1),
    [anon_sym_can] = ACTIONS(1),
    [anon_sym_LBRACK] = ACTIONS(1),
    [anon_sym_RBRACK] = ACTIONS(1),
    [anon_sym_cannot] = ACTIONS(1),
    [anon_sym_up] = ACTIONS(1),
    [anon_sym_to] = ACTIONS(1),
    [anon_sym_budget] = ACTIONS(1),
    [anon_sym_per] = ACTIONS(1),
    [anon_sym_guardrails] = ACTIONS(1),
    [anon_sym_workflow] = ACTIONS(1),
    [anon_sym_trigger] = ACTIONS(1),
    [anon_sym_stages] = ACTIONS(1),
    [anon_sym_COMMA] = ACTIONS(1),
    [anon_sym_step] = ACTIONS(1),
    [anon_sym_route] = ACTIONS(1),
    [anon_sym_on] = ACTIONS(1),
    [anon_sym_DASH_GT] = ACTIONS(1),
    [anon_sym_parallel] = ACTIONS(1),
    [anon_sym_observe] = ACTIONS(1),
    [anon_sym_fleet] = ACTIONS(1),
    [anon_sym_channel] = ACTIONS(1),
    [anon_sym_circuit_breaker] = ACTIONS(1),
    [anon_sym_DOT] = ACTIONS(1),
    [sym_currency] = ACTIONS(1),
    [sym_number] = ACTIONS(1),
    [anon_sym_DQUOTE] = ACTIONS(1),
    [anon_sym_true] = ACTIONS(1),
    [anon_sym_false] = ACTIONS(1),
  },
  [1] = {
    [sym_source_file] = STATE(122),
    [sym__definition] = STATE(4),
    [sym_import_def] = STATE(4),
    [sym_defaults_def] = STATE(4),
    [sym_provider_def] = STATE(4),
    [sym_tool_def] = STATE(4),
    [sym_archetype_def] = STATE(4),
    [sym_type_def] = STATE(4),
    [sym_policy_def] = STATE(4),
    [sym_agent_def] = STATE(4),
    [sym_workflow_def] = STATE(4),
    [sym_observe_def] = STATE(4),
    [sym_fleet_def] = STATE(4),
    [sym_channel_def] = STATE(4),
    [sym_circuit_breaker_def] = STATE(4),
    [aux_sym_source_file_repeat1] = STATE(4),
    [ts_builtin_sym_end] = ACTIONS(5),
    [sym_comment] = ACTIONS(3),
    [anon_sym_import] = ACTIONS(7),
    [anon_sym_defaults] = ACTIONS(9),
    [anon_sym_provider] = ACTIONS(11),
    [anon_sym_tool] = ACTIONS(13),
    [anon_sym_archetype] = ACTIONS(15),
    [anon_sym_type] = ACTIONS(17),
    [anon_sym_policy] = ACTIONS(19),
    [anon_sym_agent] = ACTIONS(21),
    [anon_sym_workflow] = ACTIONS(23),
    [anon_sym_observe] = ACTIONS(25),
    [anon_sym_fleet] = ACTIONS(27),
    [anon_sym_channel] = ACTIONS(29),
    [anon_sym_circuit_breaker] = ACTIONS(31),
  },
};

static const uint16_t ts_small_parse_table[] = {
  [0] = 3,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(33), 7,
      ts_builtin_sym_end,
      anon_sym_RBRACE,
      anon_sym_RBRACK,
      anon_sym_DASH_GT,
      sym_currency,
      sym_number,
      anon_sym_DQUOTE,
    ACTIONS(35), 26,
      anon_sym_import,
      anon_sym_defaults,
      anon_sym_provider,
      anon_sym_tool,
      anon_sym_archetype,
      anon_sym_type,
      anon_sym_policy,
      anon_sym_agent,
      anon_sym_model,
      anon_sym_can,
      anon_sym_cannot,
      anon_sym_budget,
      anon_sym_guardrails,
      anon_sym_workflow,
      anon_sym_trigger,
      anon_sym_stages,
      anon_sym_step,
      anon_sym_route,
      anon_sym_parallel,
      anon_sym_observe,
      anon_sym_fleet,
      anon_sym_channel,
      anon_sym_circuit_breaker,
      anon_sym_true,
      anon_sym_false,
      sym_identifier,
  [41] = 16,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(37), 1,
      ts_builtin_sym_end,
    ACTIONS(39), 1,
      anon_sym_import,
    ACTIONS(42), 1,
      anon_sym_defaults,
    ACTIONS(45), 1,
      anon_sym_provider,
    ACTIONS(48), 1,
      anon_sym_tool,
    ACTIONS(51), 1,
      anon_sym_archetype,
    ACTIONS(54), 1,
      anon_sym_type,
    ACTIONS(57), 1,
      anon_sym_policy,
    ACTIONS(60), 1,
      anon_sym_agent,
    ACTIONS(63), 1,
      anon_sym_workflow,
    ACTIONS(66), 1,
      anon_sym_observe,
    ACTIONS(69), 1,
      anon_sym_fleet,
    ACTIONS(72), 1,
      anon_sym_channel,
    ACTIONS(75), 1,
      anon_sym_circuit_breaker,
    STATE(3), 15,
      sym__definition,
      sym_import_def,
      sym_defaults_def,
      sym_provider_def,
      sym_tool_def,
      sym_archetype_def,
      sym_type_def,
      sym_policy_def,
      sym_agent_def,
      sym_workflow_def,
      sym_observe_def,
      sym_fleet_def,
      sym_channel_def,
      sym_circuit_breaker_def,
      aux_sym_source_file_repeat1,
  [104] = 16,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(7), 1,
      anon_sym_import,
    ACTIONS(9), 1,
      anon_sym_defaults,
    ACTIONS(11), 1,
      anon_sym_provider,
    ACTIONS(13), 1,
      anon_sym_tool,
    ACTIONS(15), 1,
      anon_sym_archetype,
    ACTIONS(17), 1,
      anon_sym_type,
    ACTIONS(19), 1,
      anon_sym_policy,
    ACTIONS(21), 1,
      anon_sym_agent,
    ACTIONS(23), 1,
      anon_sym_workflow,
    ACTIONS(25), 1,
      anon_sym_observe,
    ACTIONS(27), 1,
      anon_sym_fleet,
    ACTIONS(29), 1,
      anon_sym_channel,
    ACTIONS(31), 1,
      anon_sym_circuit_breaker,
    ACTIONS(78), 1,
      ts_builtin_sym_end,
    STATE(3), 15,
      sym__definition,
      sym_import_def,
      sym_defaults_def,
      sym_provider_def,
      sym_tool_def,
      sym_archetype_def,
      sym_type_def,
      sym_policy_def,
      sym_agent_def,
      sym_workflow_def,
      sym_observe_def,
      sym_fleet_def,
      sym_channel_def,
      sym_circuit_breaker_def,
      aux_sym_source_file_repeat1,
  [167] = 5,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(84), 1,
      anon_sym_DOT,
    STATE(5), 1,
      aux_sym_dotted_name_repeat1,
    ACTIONS(82), 5,
      anon_sym_RBRACE,
      anon_sym_RBRACK,
      sym_currency,
      sym_number,
      anon_sym_DQUOTE,
    ACTIONS(80), 14,
      anon_sym_model,
      anon_sym_can,
      anon_sym_cannot,
      anon_sym_up,
      anon_sym_budget,
      anon_sym_guardrails,
      anon_sym_trigger,
      anon_sym_stages,
      anon_sym_step,
      anon_sym_route,
      anon_sym_parallel,
      anon_sym_true,
      anon_sym_false,
      sym_identifier,
  [200] = 5,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(91), 1,
      anon_sym_DOT,
    STATE(5), 1,
      aux_sym_dotted_name_repeat1,
    ACTIONS(89), 5,
      anon_sym_RBRACE,
      anon_sym_RBRACK,
      sym_currency,
      sym_number,
      anon_sym_DQUOTE,
    ACTIONS(87), 14,
      anon_sym_model,
      anon_sym_can,
      anon_sym_cannot,
      anon_sym_up,
      anon_sym_budget,
      anon_sym_guardrails,
      anon_sym_trigger,
      anon_sym_stages,
      anon_sym_step,
      anon_sym_route,
      anon_sym_parallel,
      anon_sym_true,
      anon_sym_false,
      sym_identifier,
  [233] = 3,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(93), 2,
      ts_builtin_sym_end,
      anon_sym_RBRACE,
    ACTIONS(95), 19,
      anon_sym_import,
      anon_sym_defaults,
      anon_sym_provider,
      anon_sym_tool,
      anon_sym_archetype,
      anon_sym_type,
      anon_sym_policy,
      anon_sym_agent,
      anon_sym_workflow,
      anon_sym_trigger,
      anon_sym_stages,
      anon_sym_step,
      anon_sym_route,
      anon_sym_parallel,
      anon_sym_observe,
      anon_sym_fleet,
      anon_sym_channel,
      anon_sym_circuit_breaker,
      sym_identifier,
  [262] = 3,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(97), 2,
      ts_builtin_sym_end,
      anon_sym_RBRACE,
    ACTIONS(99), 19,
      anon_sym_import,
      anon_sym_defaults,
      anon_sym_provider,
      anon_sym_tool,
      anon_sym_archetype,
      anon_sym_type,
      anon_sym_policy,
      anon_sym_agent,
      anon_sym_workflow,
      anon_sym_trigger,
      anon_sym_stages,
      anon_sym_step,
      anon_sym_route,
      anon_sym_parallel,
      anon_sym_observe,
      anon_sym_fleet,
      anon_sym_channel,
      anon_sym_circuit_breaker,
      sym_identifier,
  [291] = 3,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(82), 6,
      anon_sym_RBRACE,
      anon_sym_RBRACK,
      anon_sym_DOT,
      sym_currency,
      sym_number,
      anon_sym_DQUOTE,
    ACTIONS(80), 14,
      anon_sym_model,
      anon_sym_can,
      anon_sym_cannot,
      anon_sym_up,
      anon_sym_budget,
      anon_sym_guardrails,
      anon_sym_trigger,
      anon_sym_stages,
      anon_sym_step,
      anon_sym_route,
      anon_sym_parallel,
      anon_sym_true,
      anon_sym_false,
      sym_identifier,
  [319] = 5,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(91), 1,
      anon_sym_DOT,
    STATE(6), 1,
      aux_sym_dotted_name_repeat1,
    ACTIONS(103), 5,
      anon_sym_RBRACE,
      anon_sym_RBRACK,
      sym_currency,
      sym_number,
      anon_sym_DQUOTE,
    ACTIONS(101), 13,
      anon_sym_model,
      anon_sym_can,
      anon_sym_cannot,
      anon_sym_budget,
      anon_sym_guardrails,
      anon_sym_trigger,
      anon_sym_stages,
      anon_sym_step,
      anon_sym_route,
      anon_sym_parallel,
      anon_sym_true,
      anon_sym_false,
      sym_identifier,
  [351] = 3,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(107), 5,
      anon_sym_RBRACE,
      anon_sym_RBRACK,
      sym_currency,
      sym_number,
      anon_sym_DQUOTE,
    ACTIONS(105), 13,
      anon_sym_model,
      anon_sym_can,
      anon_sym_cannot,
      anon_sym_budget,
      anon_sym_guardrails,
      anon_sym_trigger,
      anon_sym_stages,
      anon_sym_step,
      anon_sym_route,
      anon_sym_parallel,
      anon_sym_true,
      anon_sym_false,
      sym_identifier,
  [377] = 9,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(109), 1,
      sym_identifier,
    ACTIONS(111), 1,
      anon_sym_RBRACE,
    ACTIONS(113), 1,
      anon_sym_trigger,
    ACTIONS(115), 1,
      anon_sym_stages,
    ACTIONS(117), 1,
      anon_sym_step,
    ACTIONS(119), 1,
      anon_sym_route,
    ACTIONS(121), 1,
      anon_sym_parallel,
    STATE(13), 8,
      sym__workflow_field,
      sym_trigger_field,
      sym_stages_field,
      sym_step_def,
      sym_route_block,
      sym_parallel_block,
      sym_key_value,
      aux_sym_workflow_def_repeat1,
  [412] = 9,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(123), 1,
      sym_identifier,
    ACTIONS(126), 1,
      anon_sym_RBRACE,
    ACTIONS(128), 1,
      anon_sym_trigger,
    ACTIONS(131), 1,
      anon_sym_stages,
    ACTIONS(134), 1,
      anon_sym_step,
    ACTIONS(137), 1,
      anon_sym_route,
    ACTIONS(140), 1,
      anon_sym_parallel,
    STATE(13), 8,
      sym__workflow_field,
      sym_trigger_field,
      sym_stages_field,
      sym_step_def,
      sym_route_block,
      sym_parallel_block,
      sym_key_value,
      aux_sym_workflow_def_repeat1,
  [447] = 9,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(143), 1,
      sym_identifier,
    ACTIONS(146), 1,
      anon_sym_RBRACE,
    ACTIONS(148), 1,
      anon_sym_model,
    ACTIONS(151), 1,
      anon_sym_can,
    ACTIONS(154), 1,
      anon_sym_cannot,
    ACTIONS(157), 1,
      anon_sym_budget,
    ACTIONS(160), 1,
      anon_sym_guardrails,
    STATE(14), 8,
      sym__agent_field,
      sym_model_field,
      sym_can_block,
      sym_cannot_block,
      sym_budget_field,
      sym_guardrails_field,
      sym_key_value,
      aux_sym_agent_def_repeat1,
  [482] = 9,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(109), 1,
      sym_identifier,
    ACTIONS(113), 1,
      anon_sym_trigger,
    ACTIONS(115), 1,
      anon_sym_stages,
    ACTIONS(117), 1,
      anon_sym_step,
    ACTIONS(119), 1,
      anon_sym_route,
    ACTIONS(121), 1,
      anon_sym_parallel,
    ACTIONS(163), 1,
      anon_sym_RBRACE,
    STATE(12), 8,
      sym__workflow_field,
      sym_trigger_field,
      sym_stages_field,
      sym_step_def,
      sym_route_block,
      sym_parallel_block,
      sym_key_value,
      aux_sym_workflow_def_repeat1,
  [517] = 9,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(109), 1,
      sym_identifier,
    ACTIONS(165), 1,
      anon_sym_RBRACE,
    ACTIONS(167), 1,
      anon_sym_model,
    ACTIONS(169), 1,
      anon_sym_can,
    ACTIONS(171), 1,
      anon_sym_cannot,
    ACTIONS(173), 1,
      anon_sym_budget,
    ACTIONS(175), 1,
      anon_sym_guardrails,
    STATE(17), 8,
      sym__agent_field,
      sym_model_field,
      sym_can_block,
      sym_cannot_block,
      sym_budget_field,
      sym_guardrails_field,
      sym_key_value,
      aux_sym_agent_def_repeat1,
  [552] = 9,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(109), 1,
      sym_identifier,
    ACTIONS(167), 1,
      anon_sym_model,
    ACTIONS(169), 1,
      anon_sym_can,
    ACTIONS(171), 1,
      anon_sym_cannot,
    ACTIONS(173), 1,
      anon_sym_budget,
    ACTIONS(175), 1,
      anon_sym_guardrails,
    ACTIONS(177), 1,
      anon_sym_RBRACE,
    STATE(14), 8,
      sym__agent_field,
      sym_model_field,
      sym_can_block,
      sym_cannot_block,
      sym_budget_field,
      sym_guardrails_field,
      sym_key_value,
      aux_sym_agent_def_repeat1,
  [587] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(179), 14,
      ts_builtin_sym_end,
      anon_sym_import,
      anon_sym_defaults,
      anon_sym_provider,
      anon_sym_tool,
      anon_sym_archetype,
      anon_sym_type,
      anon_sym_policy,
      anon_sym_agent,
      anon_sym_workflow,
      anon_sym_observe,
      anon_sym_fleet,
      anon_sym_channel,
      anon_sym_circuit_breaker,
  [607] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(181), 14,
      ts_builtin_sym_end,
      anon_sym_import,
      anon_sym_defaults,
      anon_sym_provider,
      anon_sym_tool,
      anon_sym_archetype,
      anon_sym_type,
      anon_sym_policy,
      anon_sym_agent,
      anon_sym_workflow,
      anon_sym_observe,
      anon_sym_fleet,
      anon_sym_channel,
      anon_sym_circuit_breaker,
  [627] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(183), 14,
      ts_builtin_sym_end,
      anon_sym_import,
      anon_sym_defaults,
      anon_sym_provider,
      anon_sym_tool,
      anon_sym_archetype,
      anon_sym_type,
      anon_sym_policy,
      anon_sym_agent,
      anon_sym_workflow,
      anon_sym_observe,
      anon_sym_fleet,
      anon_sym_channel,
      anon_sym_circuit_breaker,
  [647] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(185), 14,
      ts_builtin_sym_end,
      anon_sym_import,
      anon_sym_defaults,
      anon_sym_provider,
      anon_sym_tool,
      anon_sym_archetype,
      anon_sym_type,
      anon_sym_policy,
      anon_sym_agent,
      anon_sym_workflow,
      anon_sym_observe,
      anon_sym_fleet,
      anon_sym_channel,
      anon_sym_circuit_breaker,
  [667] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(187), 14,
      ts_builtin_sym_end,
      anon_sym_import,
      anon_sym_defaults,
      anon_sym_provider,
      anon_sym_tool,
      anon_sym_archetype,
      anon_sym_type,
      anon_sym_policy,
      anon_sym_agent,
      anon_sym_workflow,
      anon_sym_observe,
      anon_sym_fleet,
      anon_sym_channel,
      anon_sym_circuit_breaker,
  [687] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(189), 14,
      ts_builtin_sym_end,
      anon_sym_import,
      anon_sym_defaults,
      anon_sym_provider,
      anon_sym_tool,
      anon_sym_archetype,
      anon_sym_type,
      anon_sym_policy,
      anon_sym_agent,
      anon_sym_workflow,
      anon_sym_observe,
      anon_sym_fleet,
      anon_sym_channel,
      anon_sym_circuit_breaker,
  [707] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(191), 14,
      ts_builtin_sym_end,
      anon_sym_import,
      anon_sym_defaults,
      anon_sym_provider,
      anon_sym_tool,
      anon_sym_archetype,
      anon_sym_type,
      anon_sym_policy,
      anon_sym_agent,
      anon_sym_workflow,
      anon_sym_observe,
      anon_sym_fleet,
      anon_sym_channel,
      anon_sym_circuit_breaker,
  [727] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(193), 14,
      ts_builtin_sym_end,
      anon_sym_import,
      anon_sym_defaults,
      anon_sym_provider,
      anon_sym_tool,
      anon_sym_archetype,
      anon_sym_type,
      anon_sym_policy,
      anon_sym_agent,
      anon_sym_workflow,
      anon_sym_observe,
      anon_sym_fleet,
      anon_sym_channel,
      anon_sym_circuit_breaker,
  [747] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(195), 14,
      ts_builtin_sym_end,
      anon_sym_import,
      anon_sym_defaults,
      anon_sym_provider,
      anon_sym_tool,
      anon_sym_archetype,
      anon_sym_type,
      anon_sym_policy,
      anon_sym_agent,
      anon_sym_workflow,
      anon_sym_observe,
      anon_sym_fleet,
      anon_sym_channel,
      anon_sym_circuit_breaker,
  [767] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(197), 14,
      ts_builtin_sym_end,
      anon_sym_import,
      anon_sym_defaults,
      anon_sym_provider,
      anon_sym_tool,
      anon_sym_archetype,
      anon_sym_type,
      anon_sym_policy,
      anon_sym_agent,
      anon_sym_workflow,
      anon_sym_observe,
      anon_sym_fleet,
      anon_sym_channel,
      anon_sym_circuit_breaker,
  [787] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(199), 14,
      ts_builtin_sym_end,
      anon_sym_import,
      anon_sym_defaults,
      anon_sym_provider,
      anon_sym_tool,
      anon_sym_archetype,
      anon_sym_type,
      anon_sym_policy,
      anon_sym_agent,
      anon_sym_workflow,
      anon_sym_observe,
      anon_sym_fleet,
      anon_sym_channel,
      anon_sym_circuit_breaker,
  [807] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(201), 14,
      ts_builtin_sym_end,
      anon_sym_import,
      anon_sym_defaults,
      anon_sym_provider,
      anon_sym_tool,
      anon_sym_archetype,
      anon_sym_type,
      anon_sym_policy,
      anon_sym_agent,
      anon_sym_workflow,
      anon_sym_observe,
      anon_sym_fleet,
      anon_sym_channel,
      anon_sym_circuit_breaker,
  [827] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(203), 14,
      ts_builtin_sym_end,
      anon_sym_import,
      anon_sym_defaults,
      anon_sym_provider,
      anon_sym_tool,
      anon_sym_archetype,
      anon_sym_type,
      anon_sym_policy,
      anon_sym_agent,
      anon_sym_workflow,
      anon_sym_observe,
      anon_sym_fleet,
      anon_sym_channel,
      anon_sym_circuit_breaker,
  [847] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(205), 14,
      ts_builtin_sym_end,
      anon_sym_import,
      anon_sym_defaults,
      anon_sym_provider,
      anon_sym_tool,
      anon_sym_archetype,
      anon_sym_type,
      anon_sym_policy,
      anon_sym_agent,
      anon_sym_workflow,
      anon_sym_observe,
      anon_sym_fleet,
      anon_sym_channel,
      anon_sym_circuit_breaker,
  [867] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(207), 14,
      ts_builtin_sym_end,
      anon_sym_import,
      anon_sym_defaults,
      anon_sym_provider,
      anon_sym_tool,
      anon_sym_archetype,
      anon_sym_type,
      anon_sym_policy,
      anon_sym_agent,
      anon_sym_workflow,
      anon_sym_observe,
      anon_sym_fleet,
      anon_sym_channel,
      anon_sym_circuit_breaker,
  [887] = 7,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(209), 1,
      sym_identifier,
    ACTIONS(212), 1,
      anon_sym_RBRACK,
    ACTIONS(217), 1,
      anon_sym_DQUOTE,
    ACTIONS(214), 2,
      sym_currency,
      sym_number,
    ACTIONS(220), 2,
      anon_sym_true,
      anon_sym_false,
    STATE(33), 5,
      sym__value,
      sym_dotted_name,
      sym_string,
      sym_boolean,
      aux_sym_list_field_repeat1,
  [915] = 7,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(223), 1,
      sym_identifier,
    ACTIONS(225), 1,
      anon_sym_RBRACK,
    ACTIONS(229), 1,
      anon_sym_DQUOTE,
    ACTIONS(227), 2,
      sym_currency,
      sym_number,
    ACTIONS(231), 2,
      anon_sym_true,
      anon_sym_false,
    STATE(36), 5,
      sym__value,
      sym_dotted_name,
      sym_string,
      sym_boolean,
      aux_sym_list_field_repeat1,
  [943] = 3,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(235), 1,
      anon_sym_RBRACE,
    ACTIONS(233), 11,
      anon_sym_model,
      anon_sym_can,
      anon_sym_cannot,
      anon_sym_budget,
      anon_sym_guardrails,
      anon_sym_trigger,
      anon_sym_stages,
      anon_sym_step,
      anon_sym_route,
      anon_sym_parallel,
      sym_identifier,
  [963] = 7,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(223), 1,
      sym_identifier,
    ACTIONS(229), 1,
      anon_sym_DQUOTE,
    ACTIONS(237), 1,
      anon_sym_RBRACK,
    ACTIONS(231), 2,
      anon_sym_true,
      anon_sym_false,
    ACTIONS(239), 2,
      sym_currency,
      sym_number,
    STATE(33), 5,
      sym__value,
      sym_dotted_name,
      sym_string,
      sym_boolean,
      aux_sym_list_field_repeat1,
  [991] = 7,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(223), 1,
      sym_identifier,
    ACTIONS(229), 1,
      anon_sym_DQUOTE,
    ACTIONS(241), 1,
      anon_sym_LBRACK,
    ACTIONS(231), 2,
      anon_sym_true,
      anon_sym_false,
    ACTIONS(243), 2,
      sym_currency,
      sym_number,
    STATE(35), 4,
      sym__value,
      sym_dotted_name,
      sym_string,
      sym_boolean,
  [1018] = 6,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(223), 1,
      sym_identifier,
    ACTIONS(229), 1,
      anon_sym_DQUOTE,
    ACTIONS(231), 2,
      anon_sym_true,
      anon_sym_false,
    ACTIONS(243), 2,
      sym_currency,
      sym_number,
    STATE(35), 4,
      sym__value,
      sym_dotted_name,
      sym_string,
      sym_boolean,
  [1042] = 4,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(245), 1,
      sym_identifier,
    ACTIONS(247), 1,
      anon_sym_RBRACE,
    STATE(53), 5,
      sym__block_item,
      sym_key_value,
      sym_list_field,
      sym_block_field,
      aux_sym_block_repeat1,
  [1059] = 3,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(251), 1,
      anon_sym_RBRACE,
    ACTIONS(249), 6,
      anon_sym_model,
      anon_sym_can,
      anon_sym_cannot,
      anon_sym_budget,
      anon_sym_guardrails,
      sym_identifier,
  [1074] = 3,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(255), 1,
      anon_sym_RBRACE,
    ACTIONS(253), 6,
      anon_sym_model,
      anon_sym_can,
      anon_sym_cannot,
      anon_sym_budget,
      anon_sym_guardrails,
      sym_identifier,
  [1089] = 3,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(259), 1,
      anon_sym_RBRACE,
    ACTIONS(257), 6,
      anon_sym_model,
      anon_sym_can,
      anon_sym_cannot,
      anon_sym_budget,
      anon_sym_guardrails,
      sym_identifier,
  [1104] = 3,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(263), 1,
      anon_sym_RBRACE,
    ACTIONS(261), 6,
      anon_sym_model,
      anon_sym_can,
      anon_sym_cannot,
      anon_sym_budget,
      anon_sym_guardrails,
      sym_identifier,
  [1119] = 3,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(267), 1,
      anon_sym_RBRACE,
    ACTIONS(265), 6,
      anon_sym_model,
      anon_sym_can,
      anon_sym_cannot,
      anon_sym_budget,
      anon_sym_guardrails,
      sym_identifier,
  [1134] = 3,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(271), 1,
      anon_sym_RBRACE,
    ACTIONS(269), 6,
      anon_sym_trigger,
      anon_sym_stages,
      anon_sym_step,
      anon_sym_route,
      anon_sym_parallel,
      sym_identifier,
  [1149] = 3,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(275), 1,
      anon_sym_RBRACE,
    ACTIONS(273), 6,
      anon_sym_trigger,
      anon_sym_stages,
      anon_sym_step,
      anon_sym_route,
      anon_sym_parallel,
      sym_identifier,
  [1164] = 3,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(279), 1,
      anon_sym_RBRACE,
    ACTIONS(277), 6,
      anon_sym_trigger,
      anon_sym_stages,
      anon_sym_step,
      anon_sym_route,
      anon_sym_parallel,
      sym_identifier,
  [1179] = 3,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(283), 1,
      anon_sym_RBRACE,
    ACTIONS(281), 6,
      anon_sym_trigger,
      anon_sym_stages,
      anon_sym_step,
      anon_sym_route,
      anon_sym_parallel,
      sym_identifier,
  [1194] = 3,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(287), 1,
      anon_sym_RBRACE,
    ACTIONS(285), 6,
      anon_sym_model,
      anon_sym_can,
      anon_sym_cannot,
      anon_sym_budget,
      anon_sym_guardrails,
      sym_identifier,
  [1209] = 4,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(245), 1,
      sym_identifier,
    ACTIONS(289), 1,
      anon_sym_RBRACE,
    STATE(39), 5,
      sym__block_item,
      sym_key_value,
      sym_list_field,
      sym_block_field,
      aux_sym_block_repeat1,
  [1226] = 3,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(293), 1,
      anon_sym_RBRACE,
    ACTIONS(291), 6,
      anon_sym_trigger,
      anon_sym_stages,
      anon_sym_step,
      anon_sym_route,
      anon_sym_parallel,
      sym_identifier,
  [1241] = 3,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(297), 1,
      anon_sym_RBRACE,
    ACTIONS(295), 6,
      anon_sym_model,
      anon_sym_can,
      anon_sym_cannot,
      anon_sym_budget,
      anon_sym_guardrails,
      sym_identifier,
  [1256] = 4,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(299), 1,
      sym_identifier,
    ACTIONS(302), 1,
      anon_sym_RBRACE,
    STATE(53), 5,
      sym__block_item,
      sym_key_value,
      sym_list_field,
      sym_block_field,
      aux_sym_block_repeat1,
  [1273] = 3,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(306), 1,
      anon_sym_RBRACE,
    ACTIONS(304), 6,
      anon_sym_model,
      anon_sym_can,
      anon_sym_cannot,
      anon_sym_budget,
      anon_sym_guardrails,
      sym_identifier,
  [1288] = 3,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(310), 1,
      anon_sym_RBRACE,
    ACTIONS(308), 6,
      anon_sym_trigger,
      anon_sym_stages,
      anon_sym_step,
      anon_sym_route,
      anon_sym_parallel,
      sym_identifier,
  [1303] = 3,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(314), 1,
      anon_sym_RBRACE,
    ACTIONS(312), 6,
      anon_sym_trigger,
      anon_sym_stages,
      anon_sym_step,
      anon_sym_route,
      anon_sym_parallel,
      sym_identifier,
  [1318] = 3,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(318), 1,
      anon_sym_RBRACE,
    ACTIONS(316), 6,
      anon_sym_trigger,
      anon_sym_stages,
      anon_sym_step,
      anon_sym_route,
      anon_sym_parallel,
      sym_identifier,
  [1333] = 5,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(229), 1,
      anon_sym_DQUOTE,
    ACTIONS(320), 1,
      anon_sym_RBRACE,
    STATE(131), 1,
      sym_string,
    STATE(64), 2,
      sym_route_arm,
      aux_sym_route_block_repeat1,
  [1350] = 5,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(322), 1,
      sym_identifier,
    ACTIONS(324), 1,
      anon_sym_RBRACK,
    STATE(67), 1,
      sym_dotted_name,
    STATE(65), 2,
      sym_capability,
      aux_sym_can_block_repeat1,
  [1367] = 5,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(229), 1,
      anon_sym_DQUOTE,
    ACTIONS(326), 1,
      anon_sym_RBRACE,
    STATE(131), 1,
      sym_string,
    STATE(58), 2,
      sym_route_arm,
      aux_sym_route_block_repeat1,
  [1384] = 5,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(322), 1,
      sym_identifier,
    ACTIONS(328), 1,
      anon_sym_RBRACK,
    STATE(67), 1,
      sym_dotted_name,
    STATE(63), 2,
      sym_capability,
      aux_sym_can_block_repeat1,
  [1401] = 5,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(330), 1,
      sym_identifier,
    ACTIONS(333), 1,
      anon_sym_RBRACK,
    STATE(67), 1,
      sym_dotted_name,
    STATE(62), 2,
      sym_capability,
      aux_sym_can_block_repeat1,
  [1418] = 5,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(322), 1,
      sym_identifier,
    ACTIONS(335), 1,
      anon_sym_RBRACK,
    STATE(67), 1,
      sym_dotted_name,
    STATE(62), 2,
      sym_capability,
      aux_sym_can_block_repeat1,
  [1435] = 5,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(337), 1,
      anon_sym_RBRACE,
    ACTIONS(339), 1,
      anon_sym_DQUOTE,
    STATE(131), 1,
      sym_string,
    STATE(64), 2,
      sym_route_arm,
      aux_sym_route_block_repeat1,
  [1452] = 5,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(322), 1,
      sym_identifier,
    ACTIONS(342), 1,
      anon_sym_RBRACK,
    STATE(67), 1,
      sym_dotted_name,
    STATE(62), 2,
      sym_capability,
      aux_sym_can_block_repeat1,
  [1469] = 4,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(344), 1,
      sym_identifier,
    STATE(66), 1,
      aux_sym_guardrails_field_repeat1,
    ACTIONS(347), 2,
      anon_sym_RBRACE,
      anon_sym_RBRACK,
  [1483] = 5,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(349), 1,
      sym_identifier,
    ACTIONS(351), 1,
      anon_sym_RBRACK,
    ACTIONS(353), 1,
      anon_sym_up,
    STATE(80), 1,
      sym_capability_constraint,
  [1499] = 4,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(355), 1,
      anon_sym_LBRACE,
    ACTIONS(357), 1,
      anon_sym_COLON,
    STATE(79), 1,
      sym_block,
  [1512] = 4,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(359), 1,
      sym_identifier,
    ACTIONS(361), 1,
      anon_sym_RBRACK,
    STATE(73), 1,
      aux_sym_guardrails_field_repeat1,
  [1525] = 4,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(363), 1,
      anon_sym_RBRACK,
    ACTIONS(365), 1,
      anon_sym_COMMA,
    STATE(75), 1,
      aux_sym_stages_field_repeat1,
  [1538] = 4,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(367), 1,
      sym_identifier,
    ACTIONS(369), 1,
      anon_sym_RBRACE,
    STATE(74), 1,
      aux_sym_guardrails_field_repeat1,
  [1551] = 4,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(365), 1,
      anon_sym_COMMA,
    ACTIONS(371), 1,
      anon_sym_RBRACK,
    STATE(70), 1,
      aux_sym_stages_field_repeat1,
  [1564] = 4,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(373), 1,
      sym_identifier,
    ACTIONS(375), 1,
      anon_sym_RBRACK,
    STATE(66), 1,
      aux_sym_guardrails_field_repeat1,
  [1577] = 4,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(373), 1,
      sym_identifier,
    ACTIONS(377), 1,
      anon_sym_RBRACE,
    STATE(66), 1,
      aux_sym_guardrails_field_repeat1,
  [1590] = 4,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(379), 1,
      anon_sym_RBRACK,
    ACTIONS(381), 1,
      anon_sym_COMMA,
    STATE(75), 1,
      aux_sym_stages_field_repeat1,
  [1603] = 3,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(355), 1,
      anon_sym_LBRACE,
    STATE(19), 1,
      sym_block,
  [1613] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(384), 2,
      anon_sym_RBRACK,
      sym_identifier,
  [1621] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(379), 2,
      anon_sym_RBRACK,
      anon_sym_COMMA,
  [1629] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(386), 2,
      anon_sym_RBRACE,
      sym_identifier,
  [1637] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(388), 2,
      anon_sym_RBRACK,
      sym_identifier,
  [1645] = 3,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(355), 1,
      anon_sym_LBRACE,
    STATE(32), 1,
      sym_block,
  [1655] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(390), 2,
      anon_sym_RBRACE,
      sym_identifier,
  [1663] = 3,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(355), 1,
      anon_sym_LBRACE,
    STATE(47), 1,
      sym_block,
  [1673] = 3,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(355), 1,
      anon_sym_LBRACE,
    STATE(30), 1,
      sym_block,
  [1683] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(392), 2,
      anon_sym_STAR,
      sym_identifier,
  [1691] = 3,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(229), 1,
      anon_sym_DQUOTE,
    STATE(26), 1,
      sym_string,
  [1701] = 3,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(355), 1,
      anon_sym_LBRACE,
    STATE(25), 1,
      sym_block,
  [1711] = 3,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(355), 1,
      anon_sym_LBRACE,
    STATE(24), 1,
      sym_block,
  [1721] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(394), 2,
      anon_sym_RBRACE,
      sym_identifier,
  [1729] = 3,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(355), 1,
      anon_sym_LBRACE,
    STATE(31), 1,
      sym_block,
  [1739] = 3,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(355), 1,
      anon_sym_LBRACE,
    STATE(23), 1,
      sym_block,
  [1749] = 3,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(355), 1,
      anon_sym_LBRACE,
    STATE(22), 1,
      sym_block,
  [1759] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(396), 2,
      anon_sym_RBRACE,
      anon_sym_DQUOTE,
  [1767] = 3,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(91), 1,
      anon_sym_DOT,
    STATE(6), 1,
      aux_sym_dotted_name_repeat1,
  [1777] = 3,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(355), 1,
      anon_sym_LBRACE,
    STATE(28), 1,
      sym_block,
  [1787] = 3,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(355), 1,
      anon_sym_LBRACE,
    STATE(21), 1,
      sym_block,
  [1797] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(398), 1,
      anon_sym_on,
  [1804] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(400), 1,
      sym_identifier,
  [1811] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(402), 1,
      anon_sym_LBRACE,
  [1818] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(404), 1,
      anon_sym_per,
  [1825] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(406), 1,
      sym_identifier,
  [1832] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(408), 1,
      sym_identifier,
  [1839] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(410), 1,
      sym_identifier,
  [1846] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(412), 1,
      sym_identifier,
  [1853] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(414), 1,
      anon_sym_LBRACE,
  [1860] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(416), 1,
      sym_identifier,
  [1867] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(418), 1,
      sym_identifier,
  [1874] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(420), 1,
      sym_identifier,
  [1881] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(422), 1,
      sym_identifier,
  [1888] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(424), 1,
      sym_identifier,
  [1895] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(426), 1,
      anon_sym_LBRACK,
  [1902] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(428), 1,
      anon_sym_to,
  [1909] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(430), 1,
      sym_identifier,
  [1916] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(432), 1,
      sym_identifier,
  [1923] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(434), 1,
      sym_identifier,
  [1930] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(436), 1,
      sym_identifier,
  [1937] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(438), 1,
      sym_identifier,
  [1944] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(440), 1,
      anon_sym_LBRACK,
  [1951] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(442), 1,
      sym_currency,
  [1958] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(444), 1,
      sym_identifier,
  [1965] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(446), 1,
      sym_identifier,
  [1972] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(448), 1,
      ts_builtin_sym_end,
  [1979] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(450), 1,
      anon_sym_from,
  [1986] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(452), 1,
      sym_currency,
  [1993] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(454), 1,
      anon_sym_DQUOTE,
  [2000] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(456), 1,
      anon_sym_LBRACE,
  [2007] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(458), 1,
      sym_identifier,
  [2014] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(460), 1,
      sym_identifier,
  [2021] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(462), 1,
      anon_sym_COLON,
  [2028] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(464), 1,
      anon_sym_COLON,
  [2035] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(466), 1,
      anon_sym_DASH_GT,
  [2042] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(468), 1,
      anon_sym_COLON,
  [2049] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(470), 1,
      anon_sym_COLON,
  [2056] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(472), 1,
      anon_sym_COLON,
  [2063] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(474), 1,
      anon_sym_LBRACK,
  [2070] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(476), 1,
      anon_sym_LBRACK,
  [2077] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(478), 1,
      sym_identifier,
  [2084] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(480), 1,
      anon_sym_COLON,
  [2091] = 2,
    ACTIONS(482), 1,
      sym_comment,
    ACTIONS(484), 1,
      aux_sym_string_token1,
  [2098] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(486), 1,
      anon_sym_LBRACE,
};

static const uint32_t ts_small_parse_table_map[] = {
  [SMALL_STATE(2)] = 0,
  [SMALL_STATE(3)] = 41,
  [SMALL_STATE(4)] = 104,
  [SMALL_STATE(5)] = 167,
  [SMALL_STATE(6)] = 200,
  [SMALL_STATE(7)] = 233,
  [SMALL_STATE(8)] = 262,
  [SMALL_STATE(9)] = 291,
  [SMALL_STATE(10)] = 319,
  [SMALL_STATE(11)] = 351,
  [SMALL_STATE(12)] = 377,
  [SMALL_STATE(13)] = 412,
  [SMALL_STATE(14)] = 447,
  [SMALL_STATE(15)] = 482,
  [SMALL_STATE(16)] = 517,
  [SMALL_STATE(17)] = 552,
  [SMALL_STATE(18)] = 587,
  [SMALL_STATE(19)] = 607,
  [SMALL_STATE(20)] = 627,
  [SMALL_STATE(21)] = 647,
  [SMALL_STATE(22)] = 667,
  [SMALL_STATE(23)] = 687,
  [SMALL_STATE(24)] = 707,
  [SMALL_STATE(25)] = 727,
  [SMALL_STATE(26)] = 747,
  [SMALL_STATE(27)] = 767,
  [SMALL_STATE(28)] = 787,
  [SMALL_STATE(29)] = 807,
  [SMALL_STATE(30)] = 827,
  [SMALL_STATE(31)] = 847,
  [SMALL_STATE(32)] = 867,
  [SMALL_STATE(33)] = 887,
  [SMALL_STATE(34)] = 915,
  [SMALL_STATE(35)] = 943,
  [SMALL_STATE(36)] = 963,
  [SMALL_STATE(37)] = 991,
  [SMALL_STATE(38)] = 1018,
  [SMALL_STATE(39)] = 1042,
  [SMALL_STATE(40)] = 1059,
  [SMALL_STATE(41)] = 1074,
  [SMALL_STATE(42)] = 1089,
  [SMALL_STATE(43)] = 1104,
  [SMALL_STATE(44)] = 1119,
  [SMALL_STATE(45)] = 1134,
  [SMALL_STATE(46)] = 1149,
  [SMALL_STATE(47)] = 1164,
  [SMALL_STATE(48)] = 1179,
  [SMALL_STATE(49)] = 1194,
  [SMALL_STATE(50)] = 1209,
  [SMALL_STATE(51)] = 1226,
  [SMALL_STATE(52)] = 1241,
  [SMALL_STATE(53)] = 1256,
  [SMALL_STATE(54)] = 1273,
  [SMALL_STATE(55)] = 1288,
  [SMALL_STATE(56)] = 1303,
  [SMALL_STATE(57)] = 1318,
  [SMALL_STATE(58)] = 1333,
  [SMALL_STATE(59)] = 1350,
  [SMALL_STATE(60)] = 1367,
  [SMALL_STATE(61)] = 1384,
  [SMALL_STATE(62)] = 1401,
  [SMALL_STATE(63)] = 1418,
  [SMALL_STATE(64)] = 1435,
  [SMALL_STATE(65)] = 1452,
  [SMALL_STATE(66)] = 1469,
  [SMALL_STATE(67)] = 1483,
  [SMALL_STATE(68)] = 1499,
  [SMALL_STATE(69)] = 1512,
  [SMALL_STATE(70)] = 1525,
  [SMALL_STATE(71)] = 1538,
  [SMALL_STATE(72)] = 1551,
  [SMALL_STATE(73)] = 1564,
  [SMALL_STATE(74)] = 1577,
  [SMALL_STATE(75)] = 1590,
  [SMALL_STATE(76)] = 1603,
  [SMALL_STATE(77)] = 1613,
  [SMALL_STATE(78)] = 1621,
  [SMALL_STATE(79)] = 1629,
  [SMALL_STATE(80)] = 1637,
  [SMALL_STATE(81)] = 1645,
  [SMALL_STATE(82)] = 1655,
  [SMALL_STATE(83)] = 1663,
  [SMALL_STATE(84)] = 1673,
  [SMALL_STATE(85)] = 1683,
  [SMALL_STATE(86)] = 1691,
  [SMALL_STATE(87)] = 1701,
  [SMALL_STATE(88)] = 1711,
  [SMALL_STATE(89)] = 1721,
  [SMALL_STATE(90)] = 1729,
  [SMALL_STATE(91)] = 1739,
  [SMALL_STATE(92)] = 1749,
  [SMALL_STATE(93)] = 1759,
  [SMALL_STATE(94)] = 1767,
  [SMALL_STATE(95)] = 1777,
  [SMALL_STATE(96)] = 1787,
  [SMALL_STATE(97)] = 1797,
  [SMALL_STATE(98)] = 1804,
  [SMALL_STATE(99)] = 1811,
  [SMALL_STATE(100)] = 1818,
  [SMALL_STATE(101)] = 1825,
  [SMALL_STATE(102)] = 1832,
  [SMALL_STATE(103)] = 1839,
  [SMALL_STATE(104)] = 1846,
  [SMALL_STATE(105)] = 1853,
  [SMALL_STATE(106)] = 1860,
  [SMALL_STATE(107)] = 1867,
  [SMALL_STATE(108)] = 1874,
  [SMALL_STATE(109)] = 1881,
  [SMALL_STATE(110)] = 1888,
  [SMALL_STATE(111)] = 1895,
  [SMALL_STATE(112)] = 1902,
  [SMALL_STATE(113)] = 1909,
  [SMALL_STATE(114)] = 1916,
  [SMALL_STATE(115)] = 1923,
  [SMALL_STATE(116)] = 1930,
  [SMALL_STATE(117)] = 1937,
  [SMALL_STATE(118)] = 1944,
  [SMALL_STATE(119)] = 1951,
  [SMALL_STATE(120)] = 1958,
  [SMALL_STATE(121)] = 1965,
  [SMALL_STATE(122)] = 1972,
  [SMALL_STATE(123)] = 1979,
  [SMALL_STATE(124)] = 1986,
  [SMALL_STATE(125)] = 1993,
  [SMALL_STATE(126)] = 2000,
  [SMALL_STATE(127)] = 2007,
  [SMALL_STATE(128)] = 2014,
  [SMALL_STATE(129)] = 2021,
  [SMALL_STATE(130)] = 2028,
  [SMALL_STATE(131)] = 2035,
  [SMALL_STATE(132)] = 2042,
  [SMALL_STATE(133)] = 2049,
  [SMALL_STATE(134)] = 2056,
  [SMALL_STATE(135)] = 2063,
  [SMALL_STATE(136)] = 2070,
  [SMALL_STATE(137)] = 2077,
  [SMALL_STATE(138)] = 2084,
  [SMALL_STATE(139)] = 2091,
  [SMALL_STATE(140)] = 2098,
};

static const TSParseActionEntry ts_parse_actions[] = {
  [0] = {.entry = {.count = 0, .reusable = false}},
  [1] = {.entry = {.count = 1, .reusable = false}}, RECOVER(),
  [3] = {.entry = {.count = 1, .reusable = true}}, SHIFT_EXTRA(),
  [5] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_source_file, 0, 0, 0),
  [7] = {.entry = {.count = 1, .reusable = true}}, SHIFT(85),
  [9] = {.entry = {.count = 1, .reusable = true}}, SHIFT(76),
  [11] = {.entry = {.count = 1, .reusable = true}}, SHIFT(110),
  [13] = {.entry = {.count = 1, .reusable = true}}, SHIFT(109),
  [15] = {.entry = {.count = 1, .reusable = true}}, SHIFT(98),
  [17] = {.entry = {.count = 1, .reusable = true}}, SHIFT(102),
  [19] = {.entry = {.count = 1, .reusable = true}}, SHIFT(104),
  [21] = {.entry = {.count = 1, .reusable = true}}, SHIFT(106),
  [23] = {.entry = {.count = 1, .reusable = true}}, SHIFT(107),
  [25] = {.entry = {.count = 1, .reusable = true}}, SHIFT(114),
  [27] = {.entry = {.count = 1, .reusable = true}}, SHIFT(115),
  [29] = {.entry = {.count = 1, .reusable = true}}, SHIFT(116),
  [31] = {.entry = {.count = 1, .reusable = true}}, SHIFT(121),
  [33] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_string, 3, 0, 0),
  [35] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_string, 3, 0, 0),
  [37] = {.entry = {.count = 1, .reusable = true}}, REDUCE(aux_sym_source_file_repeat1, 2, 0, 0),
  [39] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_source_file_repeat1, 2, 0, 0), SHIFT_REPEAT(85),
  [42] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_source_file_repeat1, 2, 0, 0), SHIFT_REPEAT(76),
  [45] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_source_file_repeat1, 2, 0, 0), SHIFT_REPEAT(110),
  [48] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_source_file_repeat1, 2, 0, 0), SHIFT_REPEAT(109),
  [51] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_source_file_repeat1, 2, 0, 0), SHIFT_REPEAT(98),
  [54] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_source_file_repeat1, 2, 0, 0), SHIFT_REPEAT(102),
  [57] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_source_file_repeat1, 2, 0, 0), SHIFT_REPEAT(104),
  [60] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_source_file_repeat1, 2, 0, 0), SHIFT_REPEAT(106),
  [63] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_source_file_repeat1, 2, 0, 0), SHIFT_REPEAT(107),
  [66] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_source_file_repeat1, 2, 0, 0), SHIFT_REPEAT(114),
  [69] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_source_file_repeat1, 2, 0, 0), SHIFT_REPEAT(115),
  [72] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_source_file_repeat1, 2, 0, 0), SHIFT_REPEAT(116),
  [75] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_source_file_repeat1, 2, 0, 0), SHIFT_REPEAT(121),
  [78] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_source_file, 1, 0, 0),
  [80] = {.entry = {.count = 1, .reusable = false}}, REDUCE(aux_sym_dotted_name_repeat1, 2, 0, 0),
  [82] = {.entry = {.count = 1, .reusable = true}}, REDUCE(aux_sym_dotted_name_repeat1, 2, 0, 0),
  [84] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_dotted_name_repeat1, 2, 0, 0), SHIFT_REPEAT(101),
  [87] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_dotted_name, 2, 0, 0),
  [89] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_dotted_name, 2, 0, 0),
  [91] = {.entry = {.count = 1, .reusable = true}}, SHIFT(101),
  [93] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_block, 2, 0, 0),
  [95] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_block, 2, 0, 0),
  [97] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_block, 3, 0, 0),
  [99] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_block, 3, 0, 0),
  [101] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym__value, 1, 0, 0),
  [103] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym__value, 1, 0, 0),
  [105] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_boolean, 1, 0, 0),
  [107] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_boolean, 1, 0, 0),
  [109] = {.entry = {.count = 1, .reusable = false}}, SHIFT(132),
  [111] = {.entry = {.count = 1, .reusable = true}}, SHIFT(18),
  [113] = {.entry = {.count = 1, .reusable = false}}, SHIFT(130),
  [115] = {.entry = {.count = 1, .reusable = false}}, SHIFT(129),
  [117] = {.entry = {.count = 1, .reusable = false}}, SHIFT(127),
  [119] = {.entry = {.count = 1, .reusable = false}}, SHIFT(97),
  [121] = {.entry = {.count = 1, .reusable = false}}, SHIFT(126),
  [123] = {.entry = {.count = 2, .reusable = false}}, REDUCE(aux_sym_workflow_def_repeat1, 2, 0, 0), SHIFT_REPEAT(132),
  [126] = {.entry = {.count = 1, .reusable = true}}, REDUCE(aux_sym_workflow_def_repeat1, 2, 0, 0),
  [128] = {.entry = {.count = 2, .reusable = false}}, REDUCE(aux_sym_workflow_def_repeat1, 2, 0, 0), SHIFT_REPEAT(130),
  [131] = {.entry = {.count = 2, .reusable = false}}, REDUCE(aux_sym_workflow_def_repeat1, 2, 0, 0), SHIFT_REPEAT(129),
  [134] = {.entry = {.count = 2, .reusable = false}}, REDUCE(aux_sym_workflow_def_repeat1, 2, 0, 0), SHIFT_REPEAT(127),
  [137] = {.entry = {.count = 2, .reusable = false}}, REDUCE(aux_sym_workflow_def_repeat1, 2, 0, 0), SHIFT_REPEAT(97),
  [140] = {.entry = {.count = 2, .reusable = false}}, REDUCE(aux_sym_workflow_def_repeat1, 2, 0, 0), SHIFT_REPEAT(126),
  [143] = {.entry = {.count = 2, .reusable = false}}, REDUCE(aux_sym_agent_def_repeat1, 2, 0, 0), SHIFT_REPEAT(132),
  [146] = {.entry = {.count = 1, .reusable = true}}, REDUCE(aux_sym_agent_def_repeat1, 2, 0, 0),
  [148] = {.entry = {.count = 2, .reusable = false}}, REDUCE(aux_sym_agent_def_repeat1, 2, 0, 0), SHIFT_REPEAT(138),
  [151] = {.entry = {.count = 2, .reusable = false}}, REDUCE(aux_sym_agent_def_repeat1, 2, 0, 0), SHIFT_REPEAT(136),
  [154] = {.entry = {.count = 2, .reusable = false}}, REDUCE(aux_sym_agent_def_repeat1, 2, 0, 0), SHIFT_REPEAT(135),
  [157] = {.entry = {.count = 2, .reusable = false}}, REDUCE(aux_sym_agent_def_repeat1, 2, 0, 0), SHIFT_REPEAT(134),
  [160] = {.entry = {.count = 2, .reusable = false}}, REDUCE(aux_sym_agent_def_repeat1, 2, 0, 0), SHIFT_REPEAT(133),
  [163] = {.entry = {.count = 1, .reusable = true}}, SHIFT(20),
  [165] = {.entry = {.count = 1, .reusable = true}}, SHIFT(29),
  [167] = {.entry = {.count = 1, .reusable = false}}, SHIFT(138),
  [169] = {.entry = {.count = 1, .reusable = false}}, SHIFT(136),
  [171] = {.entry = {.count = 1, .reusable = false}}, SHIFT(135),
  [173] = {.entry = {.count = 1, .reusable = false}}, SHIFT(134),
  [175] = {.entry = {.count = 1, .reusable = false}}, SHIFT(133),
  [177] = {.entry = {.count = 1, .reusable = true}}, SHIFT(27),
  [179] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_workflow_def, 5, 0, 1),
  [181] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_defaults_def, 2, 0, 0),
  [183] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_workflow_def, 4, 0, 1),
  [185] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_policy_def, 3, 0, 0),
  [187] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_observe_def, 3, 0, 0),
  [189] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_fleet_def, 3, 0, 0),
  [191] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_channel_def, 3, 0, 0),
  [193] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_circuit_breaker_def, 3, 0, 0),
  [195] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_import_def, 4, 0, 0),
  [197] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_agent_def, 5, 0, 1),
  [199] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_type_def, 3, 0, 0),
  [201] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_agent_def, 4, 0, 1),
  [203] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_provider_def, 3, 0, 0),
  [205] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_tool_def, 3, 0, 0),
  [207] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_archetype_def, 3, 0, 0),
  [209] = {.entry = {.count = 2, .reusable = false}}, REDUCE(aux_sym_list_field_repeat1, 2, 0, 0), SHIFT_REPEAT(10),
  [212] = {.entry = {.count = 1, .reusable = true}}, REDUCE(aux_sym_list_field_repeat1, 2, 0, 0),
  [214] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_list_field_repeat1, 2, 0, 0), SHIFT_REPEAT(33),
  [217] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_list_field_repeat1, 2, 0, 0), SHIFT_REPEAT(139),
  [220] = {.entry = {.count = 2, .reusable = false}}, REDUCE(aux_sym_list_field_repeat1, 2, 0, 0), SHIFT_REPEAT(11),
  [223] = {.entry = {.count = 1, .reusable = false}}, SHIFT(10),
  [225] = {.entry = {.count = 1, .reusable = true}}, SHIFT(89),
  [227] = {.entry = {.count = 1, .reusable = true}}, SHIFT(36),
  [229] = {.entry = {.count = 1, .reusable = true}}, SHIFT(139),
  [231] = {.entry = {.count = 1, .reusable = false}}, SHIFT(11),
  [233] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_key_value, 3, 0, 0),
  [235] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_key_value, 3, 0, 0),
  [237] = {.entry = {.count = 1, .reusable = true}}, SHIFT(82),
  [239] = {.entry = {.count = 1, .reusable = true}}, SHIFT(33),
  [241] = {.entry = {.count = 1, .reusable = true}}, SHIFT(34),
  [243] = {.entry = {.count = 1, .reusable = true}}, SHIFT(35),
  [245] = {.entry = {.count = 1, .reusable = true}}, SHIFT(68),
  [247] = {.entry = {.count = 1, .reusable = true}}, SHIFT(8),
  [249] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_cannot_block, 4, 0, 0),
  [251] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_cannot_block, 4, 0, 0),
  [253] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_cannot_block, 3, 0, 0),
  [255] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_cannot_block, 3, 0, 0),
  [257] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_can_block, 3, 0, 0),
  [259] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_can_block, 3, 0, 0),
  [261] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_model_field, 3, 0, 0),
  [263] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_model_field, 3, 0, 0),
  [265] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_guardrails_field, 5, 0, 0),
  [267] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_guardrails_field, 5, 0, 0),
  [269] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_stages_field, 5, 0, 0),
  [271] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_stages_field, 5, 0, 0),
  [273] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_trigger_field, 3, 0, 0),
  [275] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_trigger_field, 3, 0, 0),
  [277] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_step_def, 3, 0, 1),
  [279] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_step_def, 3, 0, 1),
  [281] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_parallel_block, 3, 0, 0),
  [283] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_parallel_block, 3, 0, 0),
  [285] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_can_block, 4, 0, 0),
  [287] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_can_block, 4, 0, 0),
  [289] = {.entry = {.count = 1, .reusable = true}}, SHIFT(7),
  [291] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_route_block, 6, 0, 0),
  [293] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_route_block, 6, 0, 0),
  [295] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_budget_field, 5, 0, 0),
  [297] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_budget_field, 5, 0, 0),
  [299] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_block_repeat1, 2, 0, 0), SHIFT_REPEAT(68),
  [302] = {.entry = {.count = 1, .reusable = true}}, REDUCE(aux_sym_block_repeat1, 2, 0, 0),
  [304] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_guardrails_field, 4, 0, 0),
  [306] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_guardrails_field, 4, 0, 0),
  [308] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_stages_field, 6, 0, 0),
  [310] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_stages_field, 6, 0, 0),
  [312] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_parallel_block, 4, 0, 0),
  [314] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_parallel_block, 4, 0, 0),
  [316] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_route_block, 5, 0, 0),
  [318] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_route_block, 5, 0, 0),
  [320] = {.entry = {.count = 1, .reusable = true}}, SHIFT(51),
  [322] = {.entry = {.count = 1, .reusable = true}}, SHIFT(94),
  [324] = {.entry = {.count = 1, .reusable = true}}, SHIFT(42),
  [326] = {.entry = {.count = 1, .reusable = true}}, SHIFT(57),
  [328] = {.entry = {.count = 1, .reusable = true}}, SHIFT(41),
  [330] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_can_block_repeat1, 2, 0, 0), SHIFT_REPEAT(94),
  [333] = {.entry = {.count = 1, .reusable = true}}, REDUCE(aux_sym_can_block_repeat1, 2, 0, 0),
  [335] = {.entry = {.count = 1, .reusable = true}}, SHIFT(40),
  [337] = {.entry = {.count = 1, .reusable = true}}, REDUCE(aux_sym_route_block_repeat1, 2, 0, 0),
  [339] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_route_block_repeat1, 2, 0, 0), SHIFT_REPEAT(139),
  [342] = {.entry = {.count = 1, .reusable = true}}, SHIFT(49),
  [344] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_guardrails_field_repeat1, 2, 0, 0), SHIFT_REPEAT(66),
  [347] = {.entry = {.count = 1, .reusable = true}}, REDUCE(aux_sym_guardrails_field_repeat1, 2, 0, 0),
  [349] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_capability, 1, 0, 0),
  [351] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_capability, 1, 0, 0),
  [353] = {.entry = {.count = 1, .reusable = false}}, SHIFT(112),
  [355] = {.entry = {.count = 1, .reusable = true}}, SHIFT(50),
  [357] = {.entry = {.count = 1, .reusable = true}}, SHIFT(37),
  [359] = {.entry = {.count = 1, .reusable = true}}, SHIFT(73),
  [361] = {.entry = {.count = 1, .reusable = true}}, SHIFT(54),
  [363] = {.entry = {.count = 1, .reusable = true}}, SHIFT(55),
  [365] = {.entry = {.count = 1, .reusable = true}}, SHIFT(128),
  [367] = {.entry = {.count = 1, .reusable = true}}, SHIFT(74),
  [369] = {.entry = {.count = 1, .reusable = true}}, SHIFT(48),
  [371] = {.entry = {.count = 1, .reusable = true}}, SHIFT(45),
  [373] = {.entry = {.count = 1, .reusable = true}}, SHIFT(66),
  [375] = {.entry = {.count = 1, .reusable = true}}, SHIFT(44),
  [377] = {.entry = {.count = 1, .reusable = true}}, SHIFT(56),
  [379] = {.entry = {.count = 1, .reusable = true}}, REDUCE(aux_sym_stages_field_repeat1, 2, 0, 0),
  [381] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_stages_field_repeat1, 2, 0, 0), SHIFT_REPEAT(128),
  [384] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_capability_constraint, 3, 0, 0),
  [386] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_block_field, 2, 0, 0),
  [388] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_capability, 2, 0, 0),
  [390] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_list_field, 5, 0, 0),
  [392] = {.entry = {.count = 1, .reusable = true}}, SHIFT(123),
  [394] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_list_field, 4, 0, 0),
  [396] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_route_arm, 3, 0, 0),
  [398] = {.entry = {.count = 1, .reusable = true}}, SHIFT(108),
  [400] = {.entry = {.count = 1, .reusable = true}}, SHIFT(81),
  [402] = {.entry = {.count = 1, .reusable = true}}, SHIFT(16),
  [404] = {.entry = {.count = 1, .reusable = true}}, SHIFT(117),
  [406] = {.entry = {.count = 1, .reusable = true}}, SHIFT(9),
  [408] = {.entry = {.count = 1, .reusable = true}}, SHIFT(95),
  [410] = {.entry = {.count = 1, .reusable = true}}, SHIFT(72),
  [412] = {.entry = {.count = 1, .reusable = true}}, SHIFT(96),
  [414] = {.entry = {.count = 1, .reusable = true}}, SHIFT(60),
  [416] = {.entry = {.count = 1, .reusable = true}}, SHIFT(99),
  [418] = {.entry = {.count = 1, .reusable = true}}, SHIFT(140),
  [420] = {.entry = {.count = 1, .reusable = true}}, SHIFT(105),
  [422] = {.entry = {.count = 1, .reusable = true}}, SHIFT(90),
  [424] = {.entry = {.count = 1, .reusable = true}}, SHIFT(84),
  [426] = {.entry = {.count = 1, .reusable = true}}, SHIFT(103),
  [428] = {.entry = {.count = 1, .reusable = true}}, SHIFT(124),
  [430] = {.entry = {.count = 1, .reusable = true}}, SHIFT(46),
  [432] = {.entry = {.count = 1, .reusable = true}}, SHIFT(92),
  [434] = {.entry = {.count = 1, .reusable = true}}, SHIFT(91),
  [436] = {.entry = {.count = 1, .reusable = true}}, SHIFT(88),
  [438] = {.entry = {.count = 1, .reusable = true}}, SHIFT(52),
  [440] = {.entry = {.count = 1, .reusable = true}}, SHIFT(69),
  [442] = {.entry = {.count = 1, .reusable = true}}, SHIFT(100),
  [444] = {.entry = {.count = 1, .reusable = true}}, SHIFT(43),
  [446] = {.entry = {.count = 1, .reusable = true}}, SHIFT(87),
  [448] = {.entry = {.count = 1, .reusable = true}},  ACCEPT_INPUT(),
  [450] = {.entry = {.count = 1, .reusable = true}}, SHIFT(86),
  [452] = {.entry = {.count = 1, .reusable = true}}, SHIFT(77),
  [454] = {.entry = {.count = 1, .reusable = true}}, SHIFT(2),
  [456] = {.entry = {.count = 1, .reusable = true}}, SHIFT(71),
  [458] = {.entry = {.count = 1, .reusable = true}}, SHIFT(83),
  [460] = {.entry = {.count = 1, .reusable = true}}, SHIFT(78),
  [462] = {.entry = {.count = 1, .reusable = true}}, SHIFT(111),
  [464] = {.entry = {.count = 1, .reusable = true}}, SHIFT(113),
  [466] = {.entry = {.count = 1, .reusable = true}}, SHIFT(137),
  [468] = {.entry = {.count = 1, .reusable = true}}, SHIFT(38),
  [470] = {.entry = {.count = 1, .reusable = true}}, SHIFT(118),
  [472] = {.entry = {.count = 1, .reusable = true}}, SHIFT(119),
  [474] = {.entry = {.count = 1, .reusable = true}}, SHIFT(61),
  [476] = {.entry = {.count = 1, .reusable = true}}, SHIFT(59),
  [478] = {.entry = {.count = 1, .reusable = true}}, SHIFT(93),
  [480] = {.entry = {.count = 1, .reusable = true}}, SHIFT(120),
  [482] = {.entry = {.count = 1, .reusable = false}}, SHIFT_EXTRA(),
  [484] = {.entry = {.count = 1, .reusable = false}}, SHIFT(125),
  [486] = {.entry = {.count = 1, .reusable = true}}, SHIFT(15),
};

#ifdef __cplusplus
extern "C" {
#endif
#ifdef TREE_SITTER_HIDE_SYMBOLS
#define TS_PUBLIC
#elif defined(_WIN32)
#define TS_PUBLIC __declspec(dllexport)
#else
#define TS_PUBLIC __attribute__((visibility("default")))
#endif

TS_PUBLIC const TSLanguage *tree_sitter_rein(void) {
  static const TSLanguage language = {
    .version = LANGUAGE_VERSION,
    .symbol_count = SYMBOL_COUNT,
    .alias_count = ALIAS_COUNT,
    .token_count = TOKEN_COUNT,
    .external_token_count = EXTERNAL_TOKEN_COUNT,
    .state_count = STATE_COUNT,
    .large_state_count = LARGE_STATE_COUNT,
    .production_id_count = PRODUCTION_ID_COUNT,
    .field_count = FIELD_COUNT,
    .max_alias_sequence_length = MAX_ALIAS_SEQUENCE_LENGTH,
    .parse_table = &ts_parse_table[0][0],
    .small_parse_table = ts_small_parse_table,
    .small_parse_table_map = ts_small_parse_table_map,
    .parse_actions = ts_parse_actions,
    .symbol_names = ts_symbol_names,
    .field_names = ts_field_names,
    .field_map_slices = ts_field_map_slices,
    .field_map_entries = ts_field_map_entries,
    .symbol_metadata = ts_symbol_metadata,
    .public_symbol_map = ts_symbol_map,
    .alias_map = ts_non_terminal_alias_map,
    .alias_sequences = &ts_alias_sequences[0][0],
    .lex_modes = ts_lex_modes,
    .lex_fn = ts_lex,
    .keyword_lex_fn = ts_lex_keywords,
    .keyword_capture_token = sym_identifier,
    .primary_state_ids = ts_primary_state_ids,
  };
  return &language;
}
#ifdef __cplusplus
}
#endif
