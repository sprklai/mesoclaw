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
  CheckCircle2,
  HelpCircle,
  Info,
  Loader2,
  Wifi,
  XCircle,

  // Navigation & Chrome
  ArrowRight,
  ArrowUpDown,
  ChevronDown,
  ChevronLeft,
  ChevronRight,
  ChevronUp,
  ChevronsLeft,
  ChevronsRight,
  ChevronsUpDown,
  ExternalLink,
  Home,
  Menu,
  PanelLeft,
  PanelLeftClose,
  X,

  // Actions
  Bookmark,
  Check,
  Copy,
  Download,
  Edit2,
  Eye,
  EyeOff,
  Filter,
  Maximize2,
  Minimize2,
  MoreHorizontal,
  PenLine,
  Pencil,
  Play,
  Plus,
  Power,
  RefreshCw,
  RotateCcw,
  RotateCw,
  Search,
  Send,
  Settings,
  Trash2,
  WrapText,

  // Database & Schema
  Cable,
  Calendar,
  Columns,
  Database,
  File,
  Folder,
  FolderOpen,
  Group,
  HardDrive,
  Key,
  Lock,
  Network,
  Server,
  Shield,
  Table,
  Table2,

  // Data Types & Structure
  Braces,
  Code,
  FileJson,
  FileText,
  Hash,
  Link,
  Link2,
  List,
  ListTree,
  Percent,
  Square,
  Tag,
  Type,

  // Communication & AI
  Bot,
  Brain,
  Cloud,
  Cpu,
  Globe,
  Lightbulb,
  MessageSquare,
  MessageSquarePlus,
  Sparkles,
  User,
  Zap,

  // Time & History
  Clock,
  History,

  // Analysis & Comparison
  BarChart3,
  BookOpen,
  FileSearch,
  GitCompare,
  TrendingUp,

  // Connection & Network
  WifiOff,
} from "./ui-icons";

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
  VertexAI,
  Vercel,
  XAI,
} from "./ai-icons";
