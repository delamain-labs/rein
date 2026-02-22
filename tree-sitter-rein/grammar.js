/// @file Tree-sitter grammar for the Rein agent orchestration language
/// @see https://github.com/delamain-labs/rein

module.exports = grammar({
  name: "rein",

  extras: ($) => [/\s/, $.comment],

  word: ($) => $.identifier,

  rules: {
    source_file: ($) => repeat($._definition),

    _definition: ($) =>
      choice(
        $.agent_def,
        $.workflow_def,
        $.provider_def,
        $.policy_def,
        $.type_def,
        $.import_def,
        $.defaults_def,
        $.archetype_def,
        $.tool_def,
        $.observe_def,
        $.fleet_def,
        $.channel_def,
        $.circuit_breaker_def
      ),

    // ── Comments ──────────────────────────────────────────────
    comment: ($) => token(seq("//", /.*/)),

    // ── Imports ───────────────────────────────────────────────
    import_def: ($) =>
      seq("import", choice($.identifier, "*"), "from", $.string),

    // ── Defaults ──────────────────────────────────────────────
    defaults_def: ($) => seq("defaults", $.block),

    // ── Provider ──────────────────────────────────────────────
    provider_def: ($) =>
      seq("provider", $.identifier, $.block),

    // ── Tool ──────────────────────────────────────────────────
    tool_def: ($) => seq("tool", $.identifier, $.block),

    // ── Archetype ─────────────────────────────────────────────
    archetype_def: ($) => seq("archetype", $.identifier, $.block),

    // ── Type ──────────────────────────────────────────────────
    type_def: ($) => seq("type", $.identifier, $.block),

    // ── Policy ────────────────────────────────────────────────
    policy_def: ($) => seq("policy", $.identifier, $.block),

    // ── Agent ─────────────────────────────────────────────────
    agent_def: ($) =>
      seq(
        "agent",
        field("name", $.identifier),
        "{",
        repeat($._agent_field),
        "}"
      ),

    _agent_field: ($) =>
      choice(
        $.model_field,
        $.can_block,
        $.cannot_block,
        $.budget_field,
        $.guardrails_field,
        $.key_value
      ),

    model_field: ($) => seq("model", ":", $.identifier),

    can_block: ($) =>
      seq("can", "[", repeat($.capability), "]"),

    cannot_block: ($) =>
      seq("cannot", "[", repeat($.capability), "]"),

    capability: ($) =>
      seq(
        $.dotted_name,
        optional($.capability_constraint)
      ),

    capability_constraint: ($) =>
      seq("up", "to", $.currency),

    budget_field: ($) =>
      seq("budget", ":", $.currency, "per", $.identifier),

    guardrails_field: ($) =>
      seq("guardrails", ":", "[", repeat($.identifier), "]"),

    // ── Workflow ──────────────────────────────────────────────
    workflow_def: ($) =>
      seq(
        "workflow",
        field("name", $.identifier),
        "{",
        repeat($._workflow_field),
        "}"
      ),

    _workflow_field: ($) =>
      choice(
        $.trigger_field,
        $.stages_field,
        $.step_def,
        $.route_block,
        $.parallel_block,
        $.key_value
      ),

    trigger_field: ($) => seq("trigger", ":", $.identifier),

    stages_field: ($) =>
      seq("stages", ":", "[", commaSep1($.identifier), "]"),

    step_def: ($) =>
      seq("step", field("name", $.identifier), $.block),

    route_block: ($) =>
      seq("route", "on", $.identifier, "{", repeat($.route_arm), "}"),

    route_arm: ($) =>
      seq($.string, "->", $.identifier),

    parallel_block: ($) =>
      seq("parallel", "{", repeat($.identifier), "}"),

    // ── Observe ───────────────────────────────────────────────
    observe_def: ($) => seq("observe", $.identifier, $.block),

    // ── Fleet ─────────────────────────────────────────────────
    fleet_def: ($) => seq("fleet", $.identifier, $.block),

    // ── Channel ───────────────────────────────────────────────
    channel_def: ($) => seq("channel", $.identifier, $.block),

    // ── Circuit Breaker ───────────────────────────────────────
    circuit_breaker_def: ($) =>
      seq("circuit_breaker", $.identifier, $.block),

    // ── Generic block & key-value ─────────────────────────────
    block: ($) => seq("{", repeat($._block_item), "}"),

    _block_item: ($) =>
      choice(
        $.key_value,
        $.list_field,
        $.block_field
      ),

    key_value: ($) =>
      seq($.identifier, ":", $._value),

    list_field: ($) =>
      seq($.identifier, ":", "[", repeat($._value), "]"),

    block_field: ($) =>
      seq($.identifier, $.block),

    _value: ($) =>
      choice(
        $.identifier,
        $.string,
        $.number,
        $.currency,
        $.dotted_name,
        $.boolean
      ),

    // ── Terminals ─────────────────────────────────────────────
    dotted_name: ($) =>
      seq($.identifier, repeat1(seq(".", $.identifier))),

    currency: ($) => /\$\d+(\.\d{1,2})?/,

    number: ($) => /\d+(\.\d+)?/,

    string: ($) =>
      seq('"', /[^"]*/, '"'),

    boolean: ($) => choice("true", "false"),

    identifier: ($) => /[a-zA-Z_][a-zA-Z0-9_]*/,
  },
});

/**
 * Comma-separated list with at least one element.
 */
function commaSep1(rule) {
  return seq(rule, repeat(seq(",", rule)));
}
