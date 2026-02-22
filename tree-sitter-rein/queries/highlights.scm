; Rein syntax highlighting queries for Tree-sitter

; Keywords
[
  "agent"
  "workflow"
  "provider"
  "policy"
  "type"
  "import"
  "from"
  "defaults"
  "archetype"
  "tool"
  "observe"
  "fleet"
  "channel"
  "circuit_breaker"
  "can"
  "cannot"
  "model"
  "budget"
  "per"
  "up"
  "to"
  "trigger"
  "stages"
  "step"
  "route"
  "on"
  "parallel"
  "guardrails"
] @keyword

; Boolean
(boolean) @constant.builtin

; Definition names
(agent_def name: (identifier) @type)
(workflow_def name: (identifier) @type)
(provider_def (identifier) @type)
(policy_def (identifier) @type)
(type_def (identifier) @type)
(fleet_def (identifier) @type)
(channel_def (identifier) @type)
(observe_def (identifier) @type)
(circuit_breaker_def (identifier) @type)
(archetype_def (identifier) @type)
(tool_def (identifier) @type)
(step_def name: (identifier) @function)

; Capabilities (dotted names in can/cannot blocks)
(capability (dotted_name) @function.method)

; Field names (key: value)
(key_value (identifier) @property)
(model_field "model" @property)
(budget_field "budget" @property)
(trigger_field "trigger" @property)
(stages_field "stages" @property)
(guardrails_field "guardrails" @property)

; Literals
(string) @string
(number) @number
(currency) @number

; Identifiers (fallback)
(identifier) @variable

; Comments
(comment) @comment

; Punctuation
["{" "}" "[" "]"] @punctuation.bracket
[":" "," "." "->"] @punctuation.delimiter
["*"] @operator
