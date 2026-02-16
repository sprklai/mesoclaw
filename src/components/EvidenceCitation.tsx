import { Badge } from "@/components/ui/badge";
import { Database, Columns, ArrowRight, Key, Hash } from "@/lib/icons";
import { cn } from "@/lib/utils";

export interface Evidence {
  /** Type of schema element: "table", "column", "relationship", "constraint" */
  entity_type: string;
  /** Identifier like "users.id" or "users->posts" */
  entity_id: string;
  /** Description of why this evidence is relevant */
  description: string;
}

interface EvidenceCitationProps {
  /** The evidence item */
  evidence: Evidence;
  /** Click handler for navigating to the schema element */
  onClick?: () => void;
  /** Additional CSS classes */
  className?: string;
}

/**
 * Renders a clickable citation that links to a schema element.
 * Used in explanation panels to show evidence supporting AI-generated explanations.
 */
export function EvidenceCitation({
  evidence,
  onClick,
  className,
}: EvidenceCitationProps) {
  const getIcon = () => {
    switch (evidence.entity_type.toLowerCase()) {
      case "table":
        return <Database className="h-3 w-3" />;
      case "column":
        return <Columns className="h-3 w-3" />;
      case "relationship":
        return <ArrowRight className="h-3 w-3" />;
      case "constraint":
        return <Key className="h-3 w-3" />;
      default:
        return <Hash className="h-3 w-3" />;
    }
  };

  const getVariant = (): "default" | "secondary" | "outline" => {
    switch (evidence.entity_type.toLowerCase()) {
      case "table":
        return "default";
      case "column":
        return "secondary";
      default:
        return "outline";
    }
  };

  return (
    <Badge
      className={cn(
        "cursor-pointer hover:opacity-80 transition-opacity inline-flex items-center gap-1",
        onClick && "hover:ring-2 hover:ring-ring hover:ring-offset-1",
        className
      )}
      variant={getVariant()}
      onClick={onClick}
    >
      {getIcon()}
      <span className="font-mono text-xs">{evidence.entity_id}</span>
    </Badge>
  );
}

interface EvidenceListProps {
  /** List of evidence items */
  evidence: Evidence[];
  /** Click handler for navigating to schema elements */
  onEvidenceClick?: (evidence: Evidence) => void;
  /** Additional CSS classes */
  className?: string;
  /** Maximum items to show before collapsing */
  maxItems?: number;
}

/**
 * Renders a list of evidence citations with optional collapsing.
 */
export function EvidenceList({
  evidence,
  onEvidenceClick,
  className,
  maxItems = 5,
}: EvidenceListProps) {
  const displayedEvidence = evidence.slice(0, maxItems);
  const remainingCount = evidence.length - maxItems;

  if (evidence.length === 0) {
    return (
      <p className={cn("text-sm text-muted-foreground italic", className)}>
        No supporting evidence available
      </p>
    );
  }

  return (
    <div className={cn("flex flex-wrap gap-1.5", className)}>
      {displayedEvidence.map((item, index) => (
        <EvidenceCitation
          evidence={item}
          key={`${item.entity_type}-${item.entity_id}-${index}`}
          onClick={() => onEvidenceClick?.(item)}
        />
      ))}
      {remainingCount > 0 && (
        <Badge variant="outline" className="text-xs text-muted-foreground">
          +{remainingCount} more
        </Badge>
      )}
    </div>
  );
}
