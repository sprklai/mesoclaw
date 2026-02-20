/**
 * Centralized Icon System
 *
 * This module provides a single import point for all icons used in the application.
 * Icons are organized into two categories:
 *
 * 1. **UI Icons** (lucide-react) - General UI icons for navigation, actions, status, etc.
 * 2. **AI Icons** (@lobehub/icons) - AI provider logos and branding
 *
 * @example
 * ```tsx
 * // Import any icon from the centralized system
 * import { Database, Loader2, Claude } from '@/lib/icons';
 * ```
 *
 * @module icons
 */

// ============================================
// AI PROVIDER ICONS (@lobehub/icons)
// ============================================
export {
	Anthropic,
	Claude,
	DeepSeek,
	Gemini,
	Google,
	Groq,
	Mistral,
	Ollama,
	OllamaIcon,
	OpenAI,
	OpenRouter,
	Together,
	Vercel,
	VertexAI,
	XAI,
} from "./ai-icons";
// ============================================
// TYPES
// ============================================
export type { AnyIcon, IconProps, IconType, LucideIcon } from "./types";
// ============================================
// UI ICONS (Lucide React)
// ============================================
export {
	// Status & Feedback
	AlertCircle,
	AlertTriangle,
	// Navigation & Chrome
	ArrowDown,
	ArrowRight,
	ArrowUpDown,
	// Analysis & Comparison
	BarChart3,
	// Actions
	Bookmark,
	BookOpen,
	// Communication & AI
	Bot,
	// Data Types & Structure
	Braces,
	Brain,
	// Database & Schema
	Cable,
	Calendar,
	Check,
	CheckCircle2,
	ChevronDown,
	ChevronLeft,
	ChevronRight,
	ChevronsLeft,
	ChevronsRight,
	ChevronsUpDown,
	ChevronUp,
	// Time & History
	Clock,
	Cloud,
	Code,
	Columns,
	Copy,
	Cpu,
	Database,
	Download,
	Edit2,
	ExternalLink,
	Eye,
	EyeOff,
	File,
	FileJson,
	FileSearch,
	FileText,
	Filter,
	Folder,
	FolderOpen,
	GitCompare,
	Globe,
	Group,
	HardDrive,
	Hash,
	HelpCircle,
	History,
	Home,
	Info,
	Key,
	Lightbulb,
	Link,
	Link2,
	List,
	ListTree,
	Loader2,
	Lock,
	Maximize2,
	Menu,
	MessageSquare,
	MessageSquarePlus,
	Minimize2,
	MoreHorizontal,
	Network,
	PanelLeft,
	PanelLeftClose,
	Pause,
	Pencil,
	PenLine,
	Percent,
	Play,
	Plus,
	Power,
	RefreshCw,
	RotateCcw,
	RotateCw,
	ScrollText,
	Search,
	Send,
	Server,
	Settings,
	Shield,
	Sparkles,
	Square,
	Table,
	Table2,
	Tag,
	Trash2,
	TrendingUp,
	Type,
	User,
	Wand2,
	Wifi,
	// Connection & Network
	WifiOff,
	WrapText,
	X,
	XCircle,
	Zap,
} from "./ui-icons";
