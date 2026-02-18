/**
 * AI SDK Elements
 *
 * Reusable wrapper components based on AI SDK Elements design patterns.
 * These components are adapted for use in a Tauri + React application.
 *
 * Documentation: https://ai-sdk.dev/elements
 */

// Conversation components
export {
  Conversation,
  ConversationContent,
  ConversationEmptyState,
  ConversationScrollButton,
} from "./conversation";

// PromptInput components
export {
  PromptInput,
  PromptInputTextarea,
  PromptInputSubmit,
  PromptInputBody,
  PromptInputFooter,
  PromptInputHeader,
  PromptInputTools,
  PromptInputButton,
  PromptInputActionMenu,
  PromptInputActionMenuTrigger,
  PromptInputActionMenuContent,
  PromptInputActionAddAttachments,
  usePromptInputAttachments,
} from "./prompt-input";

// Message components
export {
  Message,
  MessageContent,
  MessageResponse,
  MessageBranch,
  MessageBranchContent,
  MessageBranchSelector,
  MessageBranchPrevious,
  MessageBranchNext,
  MessageBranchPage,
} from "./message";

// Suggestion components
export { Suggestion, Suggestions } from "./suggestion";

// Attachments components
export {
  Attachments,
  Attachment,
  AttachmentPreview,
  AttachmentRemove,
} from "./attachments";

// Sources components
export {
  Sources,
  SourcesTrigger,
  SourcesContent,
  Source,
} from "./sources";

// Reasoning components
export {
  Reasoning,
  ReasoningTrigger,
  ReasoningContent,
} from "./reasoning";

// Speech input component
export { SpeechInput } from "./speech-input";

// CodeBlock components
export { CodeBlock, CodeBlockCopyButton } from "./code-block";

// Artifact components
export {
  Artifact,
  ArtifactHeader,
  ArtifactTitle,
  ArtifactDescription,
  ArtifactActions,
  ArtifactAction,
  ArtifactClose,
  ArtifactContent,
} from "./artifact";
