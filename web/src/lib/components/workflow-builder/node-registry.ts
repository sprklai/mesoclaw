// node-registry.ts — Single source of truth for workflow builder node types.
// Pure TypeScript, zero framework imports. Icons are string names (lucide).
// Adding a new node = adding one object to NODE_DEFINITIONS.

// ---------------------------------------------------------------------------
// Interfaces
// ---------------------------------------------------------------------------

export interface NodeFieldDef {
	key: string;
	label: string;
	type: 'text' | 'textarea' | 'number' | 'select' | 'boolean' | 'json' | 'step-ref' | 'step-refs';
	options?: { value: string; label: string }[];
	default?: unknown;
	placeholder?: string;
	required?: boolean;
	description?: string;
}

export interface HandleDef {
	id: string;
	label?: string;
	type: 'source' | 'target';
	position: 'top' | 'bottom' | 'left' | 'right';
}

export type NodeVisual = 'standard' | 'condition' | 'trigger';

export type NodeCategory =
	| 'triggers'
	| 'ai'
	| 'search'
	| 'system'
	| 'files'
	| 'memory'
	| 'channels'
	| 'config'
	| 'schedule'
	| 'flow';

export interface NodeDefinition {
	type: string;
	label: string;
	category: NodeCategory;
	icon: string;
	description: string;
	visual: NodeVisual;
	fields: NodeFieldDef[];
	handles: HandleDef[];
	featureGate?: string;
	hidden?: boolean;
	fromStep(step: Record<string, unknown>): Record<string, unknown>;
	toStep(data: Record<string, unknown>): Record<string, unknown>;
}

// ---------------------------------------------------------------------------
// Standard handle presets
// ---------------------------------------------------------------------------

const STANDARD_HANDLES: HandleDef[] = [
	{ id: 'target', type: 'target', position: 'top' },
	{ id: 'source', type: 'source', position: 'bottom' }
];

const TRIGGER_HANDLES: HandleDef[] = [
	{ id: 'source', type: 'source', position: 'bottom' }
];

const CONDITION_HANDLES: HandleDef[] = [
	{ id: 'target', type: 'target', position: 'top' },
	{ id: 'true', label: 'wb_handle_true', type: 'source', position: 'right' },
	{ id: 'false', label: 'wb_handle_false', type: 'source', position: 'bottom' }
];

export { STANDARD_HANDLES, TRIGGER_HANDLES, CONDITION_HANDLES };

// ---------------------------------------------------------------------------
// Helpers — fromStep / toStep factories
// ---------------------------------------------------------------------------

type StepRec = Record<string, unknown>;

/** Extract fields from `step.args` (tool-type nodes). */
function toolFromStep(keys: string[]) {
	return (step: StepRec): StepRec => {
		const args = (step.args ?? {}) as StepRec;
		const out: StepRec = {};
		for (const k of keys) {
			if (args[k] !== undefined) out[k] = args[k];
		}
		return out;
	};
}

/** Pack fields into `{ type: 'tool', tool, args }`. */
function toolToStep(tool: string, keys: string[]) {
	return (data: StepRec): StepRec => {
		const args: StepRec = {};
		for (const k of keys) {
			if (data[k] !== undefined) args[k] = data[k];
		}
		return { type: 'tool', tool, args };
	};
}

/** Trigger nodes pack args with an extra `trigger_type` field. */
function triggerToStep(triggerType: string, keys: string[]) {
	return (data: StepRec): StepRec => {
		const args: StepRec = { trigger_type: triggerType };
		for (const k of keys) {
			if (data[k] !== undefined) args[k] = data[k];
		}
		return { type: 'tool', tool: 'trigger', args };
	};
}

// ---------------------------------------------------------------------------
// Config key options (whitelisted)
// ---------------------------------------------------------------------------

const CONFIG_KEY_OPTIONS = [
	{ value: 'context_injection_enabled', label: 'wb_option_context_injection_enabled' },
	{ value: 'self_evolution_enabled', label: 'wb_option_self_evolution_enabled' },
	{ value: 'learning_enabled', label: 'wb_option_learning_enabled' },
	{ value: 'agent_system_prompt', label: 'wb_option_agent_system_prompt' }
];

// ---------------------------------------------------------------------------
// Node definitions
// ---------------------------------------------------------------------------

export const NODE_DEFINITIONS: NodeDefinition[] = [
	// ── Triggers ──────────────────────────────────────────────────────────
	// Hidden: trigger nodes are visual-only — schedule is set via toolbar metadata
	{
		type: 'trigger_cron',
		label: 'wb_node_trigger_cron_label',
		category: 'triggers',
		icon: 'Clock',
		description: 'wb_node_trigger_cron_description',
		visual: 'trigger',
		hidden: true,
		fields: [
			{
				key: 'cron_expr',
				label: 'wb_field_cron_expr_label',
				type: 'text',
				required: true,
				placeholder: 'wb_field_cron_expr_placeholder',
				description: 'wb_field_cron_expr_description'
			}
		],
		handles: TRIGGER_HANDLES,
		fromStep(step) {
			const args = (step.args ?? {}) as StepRec;
			const out: StepRec = {};
			if (args.cron_expr !== undefined) out.cron_expr = args.cron_expr;
			return out;
		},
		toStep: triggerToStep('cron', ['cron_expr'])
	},
	{
		type: 'trigger_manual',
		label: 'wb_node_trigger_manual_label',
		category: 'triggers',
		icon: 'PlayCircle',
		description: 'wb_node_trigger_manual_description',
		visual: 'trigger',
		hidden: true,
		fields: [],
		handles: TRIGGER_HANDLES,
		fromStep() {
			return {};
		},
		toStep: triggerToStep('manual', [])
	},

	// ── AI ────────────────────────────────────────────────────────────────
	{
		type: 'llm',
		label: 'wb_node_llm_label',
		category: 'ai',
		icon: 'Brain',
		description: 'wb_node_llm_description',
		visual: 'standard',
		fields: [
			{
				key: 'prompt',
				label: 'wb_field_prompt_label',
				type: 'textarea',
				required: true,
				placeholder: 'wb_field_prompt_placeholder',
				description: 'wb_field_prompt_description'
			},
			{
				key: 'model',
				label: 'wb_field_model_label',
				type: 'text',
				placeholder: 'wb_field_model_placeholder',
				description: 'wb_field_model_description'
			}
		],
		handles: STANDARD_HANDLES,
		fromStep(step) {
			const out: StepRec = {};
			if (step.prompt !== undefined) out.prompt = step.prompt;
			if (step.model !== undefined) out.model = step.model;
			return out;
		},
		toStep(data) {
			const out: StepRec = { type: 'llm' };
			if (data.prompt !== undefined) out.prompt = data.prompt;
			if (data.model !== undefined) out.model = data.model;
			return out;
		}
	},
	{
		type: 'agent_turn',
		label: 'wb_node_agent_turn_label',
		category: 'ai',
		icon: 'Bot',
		description: 'wb_node_agent_turn_description',
		visual: 'standard',
		fields: [
			{
				key: 'prompt',
				label: 'wb_field_prompt_label',
				type: 'textarea',
				required: true,
				placeholder: 'wb_field_agent_prompt_placeholder',
				description: 'wb_field_agent_prompt_description'
			},
			{
				key: 'session_target',
				label: 'wb_field_session_target_label',
				type: 'select',
				options: [
					{ value: 'main', label: 'wb_option_session_main' },
					{ value: 'isolated', label: 'wb_option_session_isolated' }
				],
				description: 'wb_field_session_target_description'
			}
		],
		handles: STANDARD_HANDLES,
		fromStep: toolFromStep(['prompt', 'session_target']),
		toStep: toolToStep('agent_turn', ['prompt', 'session_target'])
	},

	// ── Search ────────────────────────────────────────────────────────────
	{
		type: 'web_search',
		label: 'wb_node_web_search_label',
		category: 'search',
		icon: 'Globe',
		description: 'wb_node_web_search_description',
		visual: 'standard',
		fields: [
			{
				key: 'query',
				label: 'wb_field_query_label',
				type: 'text',
				required: true,
				placeholder: 'wb_field_search_query_placeholder',
				description: 'wb_field_search_query_description'
			},
			{
				key: 'max_results',
				label: 'wb_field_max_results_label',
				type: 'number',
				default: 5,
				description: 'wb_field_max_results_description'
			}
		],
		handles: STANDARD_HANDLES,
		fromStep: toolFromStep(['query', 'max_results']),
		toStep: toolToStep('web_search', ['query', 'max_results'])
	},

	// ── System ────────────────────────────────────────────────────────────
	{
		type: 'system_info',
		label: 'wb_node_system_info_label',
		category: 'system',
		icon: 'Monitor',
		description: 'wb_node_system_info_description',
		visual: 'standard',
		fields: [
			{
				key: 'action',
				label: 'wb_field_action_label',
				type: 'select',
				options: [
					{ value: 'os', label: 'wb_option_action_os' },
					{ value: 'cpu', label: 'wb_option_action_cpu' },
					{ value: 'memory', label: 'wb_option_action_memory' },
					{ value: 'disk', label: 'wb_option_action_disk' },
					{ value: 'network', label: 'wb_option_action_network' },
					{ value: 'processes', label: 'wb_option_action_processes' },
					{ value: 'all', label: 'wb_option_action_all' }
				],
				description: 'wb_field_sysinfo_action_description'
			}
		],
		handles: STANDARD_HANDLES,
		fromStep: toolFromStep(['action']),
		toStep: toolToStep('system_info', ['action'])
	},
	{
		type: 'shell',
		label: 'wb_node_shell_label',
		category: 'system',
		icon: 'Terminal',
		description: 'wb_node_shell_description',
		visual: 'standard',
		fields: [
			{
				key: 'command',
				label: 'wb_field_command_label',
				type: 'text',
				required: true,
				placeholder: 'wb_field_command_placeholder',
				description: 'wb_field_command_description'
			},
			{
				key: 'timeout_secs',
				label: 'wb_field_timeout_secs_label',
				type: 'number',
				placeholder: 'wb_field_timeout_secs_placeholder',
				description: 'wb_field_timeout_secs_description'
			}
		],
		handles: STANDARD_HANDLES,
		fromStep: toolFromStep(['command', 'timeout_secs']),
		toStep: toolToStep('shell', ['command', 'timeout_secs'])
	},
	{
		type: 'process',
		label: 'wb_node_process_label',
		category: 'system',
		icon: 'Activity',
		description: 'wb_node_process_description',
		visual: 'standard',
		fields: [
			{
				key: 'action',
				label: 'wb_field_action_label',
				type: 'select',
				options: [
					{ value: 'list', label: 'wb_option_process_list' },
					{ value: 'find', label: 'wb_option_process_find' },
					{ value: 'kill', label: 'wb_option_process_kill' }
				],
				description: 'wb_field_process_action_description'
			},
			{
				key: 'target',
				label: 'wb_field_target_label',
				type: 'text',
				placeholder: 'wb_field_process_target_placeholder',
				description: 'wb_field_process_target_description'
			}
		],
		handles: STANDARD_HANDLES,
		fromStep: toolFromStep(['action', 'target']),
		toStep: toolToStep('process', ['action', 'target'])
	},

	// ── Files ─────────────────────────────────────────────────────────────
	{
		type: 'file_read',
		label: 'wb_node_file_read_label',
		category: 'files',
		icon: 'FileText',
		description: 'wb_node_file_read_description',
		visual: 'standard',
		fields: [
			{
				key: 'path',
				label: 'wb_field_path_label',
				type: 'text',
				required: true,
				placeholder: 'wb_field_file_path_placeholder',
				description: 'wb_field_file_read_path_description'
			}
		],
		handles: STANDARD_HANDLES,
		fromStep: toolFromStep(['path']),
		toStep: toolToStep('file_read', ['path'])
	},
	{
		type: 'file_write',
		label: 'wb_node_file_write_label',
		category: 'files',
		icon: 'FilePlus',
		description: 'wb_node_file_write_description',
		visual: 'standard',
		fields: [
			{
				key: 'path',
				label: 'wb_field_path_label',
				type: 'text',
				required: true,
				placeholder: 'wb_field_file_path_placeholder',
				description: 'wb_field_file_write_path_description'
			},
			{
				key: 'content',
				label: 'wb_field_content_label',
				type: 'textarea',
				placeholder: 'wb_field_file_content_placeholder',
				description: 'wb_field_file_content_description'
			}
		],
		handles: STANDARD_HANDLES,
		fromStep: toolFromStep(['path', 'content']),
		toStep: toolToStep('file_write', ['path', 'content'])
	},
	{
		type: 'file_search',
		label: 'wb_node_file_search_label',
		category: 'files',
		icon: 'Search',
		description: 'wb_node_file_search_description',
		visual: 'standard',
		fields: [
			{
				key: 'pattern',
				label: 'wb_field_pattern_label',
				type: 'text',
				required: true,
				placeholder: 'wb_field_search_pattern_placeholder',
				description: 'wb_field_search_pattern_description'
			},
			{
				key: 'path',
				label: 'wb_field_path_label',
				type: 'text',
				placeholder: 'wb_field_search_path_placeholder',
				description: 'wb_field_search_path_description'
			},
			{
				key: 'max_results',
				label: 'wb_field_max_results_label',
				type: 'number',
				description: 'wb_field_file_search_max_results_description'
			}
		],
		handles: STANDARD_HANDLES,
		fromStep: toolFromStep(['pattern', 'path', 'max_results']),
		toStep: toolToStep('file_search', ['pattern', 'path', 'max_results'])
	},
	{
		type: 'file_list',
		label: 'wb_node_file_list_label',
		category: 'files',
		icon: 'FolderOpen',
		description: 'wb_node_file_list_description',
		visual: 'standard',
		fields: [
			{
				key: 'path',
				label: 'wb_field_path_label',
				type: 'text',
				required: true,
				placeholder: 'wb_field_dir_path_placeholder',
				description: 'wb_field_file_list_path_description'
			},
			{
				key: 'recursive',
				label: 'wb_field_recursive_label',
				type: 'boolean',
				description: 'wb_field_recursive_description'
			}
		],
		handles: STANDARD_HANDLES,
		fromStep: toolFromStep(['path', 'recursive']),
		toStep: toolToStep('file_list', ['path', 'recursive'])
	},
	{
		type: 'patch',
		label: 'wb_node_patch_label',
		category: 'files',
		icon: 'FileDiff',
		description: 'wb_node_patch_description',
		visual: 'standard',
		fields: [
			{
				key: 'path',
				label: 'wb_field_path_label',
				type: 'text',
				required: true,
				placeholder: 'wb_field_file_path_placeholder',
				description: 'wb_field_patch_path_description'
			},
			{
				key: 'original',
				label: 'wb_field_original_label',
				type: 'textarea',
				placeholder: 'wb_field_original_placeholder',
				description: 'wb_field_original_description'
			},
			{
				key: 'replacement',
				label: 'wb_field_replacement_label',
				type: 'textarea',
				placeholder: 'wb_field_replacement_placeholder',
				description: 'wb_field_replacement_description'
			}
		],
		handles: STANDARD_HANDLES,
		fromStep: toolFromStep(['path', 'original', 'replacement']),
		toStep: toolToStep('patch', ['path', 'original', 'replacement'])
	},

	// ── Memory ────────────────────────────────────────────────────────────
	{
		type: 'memory_store',
		label: 'wb_node_memory_store_label',
		category: 'memory',
		icon: 'Database',
		description: 'wb_node_memory_store_description',
		visual: 'standard',
		fields: [
			{
				key: 'content',
				label: 'wb_field_content_label',
				type: 'textarea',
				required: true,
				placeholder: 'wb_field_memory_content_placeholder',
				description: 'wb_field_memory_store_content_description'
			},
			{
				key: 'tags',
				label: 'wb_field_tags_label',
				type: 'text',
				placeholder: 'wb_field_tags_placeholder',
				description: 'wb_field_tags_description'
			}
		],
		handles: STANDARD_HANDLES,
		fromStep: toolFromStep(['content', 'tags']),
		toStep: toolToStep('memory_store', ['content', 'tags'])
	},
	{
		type: 'memory_recall',
		label: 'wb_node_memory_recall_label',
		category: 'memory',
		icon: 'SearchCode',
		description: 'wb_node_memory_recall_description',
		visual: 'standard',
		fields: [
			{
				key: 'query',
				label: 'wb_field_query_label',
				type: 'text',
				required: true,
				placeholder: 'wb_field_recall_query_placeholder',
				description: 'wb_field_recall_query_description'
			},
			{
				key: 'limit',
				label: 'wb_field_limit_label',
				type: 'number',
				default: 10,
				description: 'wb_field_recall_limit_description'
			}
		],
		handles: STANDARD_HANDLES,
		fromStep: toolFromStep(['query', 'limit']),
		toStep: toolToStep('memory_recall', ['query', 'limit'])
	},
	{
		type: 'memory_forget',
		label: 'wb_node_memory_forget_label',
		category: 'memory',
		icon: 'Trash2',
		description: 'wb_node_memory_forget_description',
		visual: 'standard',
		fields: [
			{
				key: 'memory_id',
				label: 'wb_field_memory_id_label',
				type: 'text',
				required: true,
				placeholder: 'wb_field_memory_id_placeholder',
				description: 'wb_field_memory_id_description'
			}
		],
		handles: STANDARD_HANDLES,
		fromStep: toolFromStep(['memory_id']),
		toStep: toolToStep('memory_forget', ['memory_id'])
	},

	// ── Channels ──────────────────────────────────────────────────────────
	{
		type: 'channel_send',
		label: 'wb_node_channel_send_label',
		category: 'channels',
		icon: 'Send',
		description: 'wb_node_channel_send_description',
		visual: 'standard',
		fields: [
			{
				key: 'channel',
				label: 'wb_field_channel_label',
				type: 'text',
				required: true,
				placeholder: 'wb_field_channel_placeholder',
				description: 'wb_field_channel_description'
			},
			{
				key: 'message',
				label: 'wb_field_message_label',
				type: 'textarea',
				required: true,
				placeholder: 'wb_field_channel_message_placeholder',
				description: 'wb_field_channel_message_description'
			}
		],
		handles: STANDARD_HANDLES,
		featureGate: 'channels',
		fromStep: toolFromStep(['channel', 'message']),
		toStep: toolToStep('channel_send', ['channel', 'message'])
	},
	{
		type: 'notify',
		label: 'wb_node_notify_label',
		category: 'channels',
		icon: 'Bell',
		description: 'wb_node_notify_description',
		visual: 'standard',
		fields: [
			{
				key: 'title',
				label: 'wb_field_title_label',
				type: 'text',
				required: true,
				placeholder: 'wb_field_notify_title_placeholder',
				description: 'wb_field_notify_title_description'
			},
			{
				key: 'body',
				label: 'wb_field_body_label',
				type: 'textarea',
				placeholder: 'wb_field_notify_body_placeholder',
				description: 'wb_field_notify_body_description'
			},
			{
				key: 'level',
				label: 'wb_field_level_label',
				type: 'select',
				options: [
					{ value: 'info', label: 'wb_option_level_info' },
					{ value: 'warn', label: 'wb_option_level_warn' },
					{ value: 'error', label: 'wb_option_level_error' }
				],
				description: 'wb_field_notify_level_description'
			}
		],
		handles: STANDARD_HANDLES,
		fromStep: toolFromStep(['title', 'body', 'level']),
		toStep: toolToStep('notify', ['title', 'body', 'level'])
	},

	// ── Config ────────────────────────────────────────────────────────────
	{
		type: 'config_read',
		label: 'wb_node_config_read_label',
		category: 'config',
		icon: 'Settings',
		description: 'wb_node_config_read_description',
		visual: 'standard',
		fields: [
			{
				key: 'key',
				label: 'wb_field_config_key_label',
				type: 'select',
				options: CONFIG_KEY_OPTIONS,
				description: 'wb_field_config_key_description'
			}
		],
		handles: STANDARD_HANDLES,
		fromStep: toolFromStep(['key']),
		toStep: toolToStep('config_read', ['key'])
	},
	{
		type: 'config_update',
		label: 'wb_node_config_update_label',
		category: 'config',
		icon: 'Settings2',
		description: 'wb_node_config_update_description',
		visual: 'standard',
		fields: [
			{
				key: 'key',
				label: 'wb_field_config_key_label',
				type: 'select',
				options: CONFIG_KEY_OPTIONS,
				description: 'wb_field_config_key_description'
			},
			{
				key: 'value',
				label: 'wb_field_value_label',
				type: 'text',
				required: true,
				placeholder: 'wb_field_config_value_placeholder',
				description: 'wb_field_config_value_description'
			}
		],
		handles: STANDARD_HANDLES,
		fromStep: toolFromStep(['key', 'value']),
		toStep: toolToStep('config_update', ['key', 'value'])
	},

	// ── Schedule ──────────────────────────────────────────────────────────
	{
		type: 'create_job',
		label: 'wb_node_create_job_label',
		category: 'schedule',
		icon: 'CalendarPlus',
		description: 'wb_node_create_job_description',
		visual: 'standard',
		fields: [
			{
				key: 'name',
				label: 'wb_field_job_name_label',
				type: 'text',
				required: true,
				placeholder: 'wb_field_job_name_placeholder',
				description: 'wb_field_job_name_description'
			},
			{
				key: 'schedule',
				label: 'wb_field_schedule_label',
				type: 'text',
				required: true,
				placeholder: 'wb_field_schedule_placeholder',
				description: 'wb_field_schedule_description'
			},
			{
				key: 'payload_type',
				label: 'wb_field_payload_type_label',
				type: 'select',
				options: [
					{ value: 'heartbeat', label: 'wb_option_payload_heartbeat' },
					{ value: 'agent_turn', label: 'wb_option_payload_agent_turn' },
					{ value: 'notify', label: 'wb_option_payload_notify' },
					{ value: 'send_via_channel', label: 'wb_option_payload_send_via_channel' },
					{ value: 'workflow', label: 'wb_option_payload_workflow' }
				],
				description: 'wb_field_payload_type_description'
			}
		],
		handles: STANDARD_HANDLES,
		fromStep: toolFromStep(['name', 'schedule', 'payload_type']),
		toStep: toolToStep('create_job', ['name', 'schedule', 'payload_type'])
	},
	{
		type: 'toggle_job',
		label: 'wb_node_toggle_job_label',
		category: 'schedule',
		icon: 'ToggleLeft',
		description: 'wb_node_toggle_job_description',
		visual: 'standard',
		fields: [
			{
				key: 'job_name',
				label: 'wb_field_job_name_label',
				type: 'text',
				required: true,
				placeholder: 'wb_field_job_name_placeholder',
				description: 'wb_field_toggle_job_name_description'
			},
			{
				key: 'enabled',
				label: 'wb_field_enabled_label',
				type: 'boolean',
				description: 'wb_field_enabled_description'
			}
		],
		handles: STANDARD_HANDLES,
		fromStep: toolFromStep(['job_name', 'enabled']),
		toStep: toolToStep('toggle_job', ['job_name', 'enabled'])
	},

	// ── Flow ──────────────────────────────────────────────────────────────
	{
		type: 'delay',
		label: 'wb_node_delay_label',
		category: 'flow',
		icon: 'Timer',
		description: 'wb_node_delay_description',
		visual: 'standard',
		fields: [
			{
				key: 'seconds',
				label: 'wb_field_seconds_label',
				type: 'number',
				required: true,
				placeholder: 'wb_field_seconds_placeholder',
				description: 'wb_field_delay_seconds_description'
			}
		],
		handles: STANDARD_HANDLES,
		fromStep(step) {
			const out: StepRec = {};
			if (step.seconds !== undefined) out.seconds = step.seconds;
			return out;
		},
		toStep(data) {
			const out: StepRec = { type: 'delay' };
			if (data.seconds !== undefined) out.seconds = data.seconds;
			return out;
		}
	},
	{
		type: 'condition',
		label: 'wb_node_condition_label',
		category: 'flow',
		icon: 'GitBranch',
		description: 'wb_node_condition_description',
		visual: 'condition',
		fields: [
			{
				key: 'expression',
				label: 'wb_field_expression_label',
				type: 'text',
				required: true,
				placeholder: 'wb_field_expression_placeholder',
				description: 'wb_field_expression_description'
			},
			{
				key: 'if_true',
				label: 'wb_field_if_true_label',
				type: 'step-ref',
				description: 'wb_field_if_true_description'
			},
			{
				key: 'if_false',
				label: 'wb_field_if_false_label',
				type: 'step-ref',
				description: 'wb_field_if_false_description'
			}
		],
		handles: CONDITION_HANDLES,
		fromStep(step) {
			const out: StepRec = {};
			if (step.expression !== undefined) out.expression = step.expression;
			if (step.if_true !== undefined) out.if_true = step.if_true;
			if (step.if_false !== undefined) out.if_false = step.if_false;
			return out;
		},
		toStep(data) {
			const out: StepRec = { type: 'condition' };
			if (data.expression !== undefined) out.expression = data.expression;
			if (data.if_true !== undefined) out.if_true = data.if_true;
			if (data.if_false !== undefined) out.if_false = data.if_false;
			return out;
		}
	},
	// Hidden: backend parallel execution not yet implemented (executor.rs:349)
	{
		type: 'parallel',
		label: 'wb_node_parallel_label',
		category: 'flow',
		icon: 'GitFork',
		description: 'wb_node_parallel_description',
		visual: 'standard',
		hidden: true,
		fields: [
			{
				key: 'steps',
				label: 'wb_field_steps_label',
				type: 'step-refs',
				required: true,
				description: 'wb_field_parallel_steps_description'
			}
		],
		handles: STANDARD_HANDLES,
		fromStep(step) {
			const out: StepRec = {};
			if (step.steps !== undefined) out.steps = step.steps;
			return out;
		},
		toStep(data) {
			const out: StepRec = { type: 'parallel' };
			if (data.steps !== undefined) out.steps = data.steps;
			return out;
		}
	}
];

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

const byType = new Map<string, NodeDefinition>();
for (const def of NODE_DEFINITIONS) {
	byType.set(def.type, def);
}

export const nodeRegistry = {
	/** Return all node definitions. */
	getAll(): NodeDefinition[] {
		return NODE_DEFINITIONS;
	},

	/** Look up a single node definition by its type key. */
	get(type: string): NodeDefinition | undefined {
		return byType.get(type);
	},

	/** Return definitions grouped by category. */
	getByCategory(): Map<NodeCategory, NodeDefinition[]> {
		const map = new Map<NodeCategory, NodeDefinition[]>();
		for (const def of NODE_DEFINITIONS) {
			const list = map.get(def.category);
			if (list) {
				list.push(def);
			} else {
				map.set(def.category, [def]);
			}
		}
		return map;
	},

	/**
	 * Return definitions visible given the active capability/feature set.
	 * Nodes without a `featureGate` are always visible.
	 * Nodes with a `featureGate` are visible only if that gate is present
	 * in the provided `capabilities` array.
	 */
	getVisible(capabilities: string[]): NodeDefinition[] {
		const caps = new Set(capabilities);
		return NODE_DEFINITIONS.filter((def) => !def.hidden && (!def.featureGate || caps.has(def.featureGate)));
	}
};
