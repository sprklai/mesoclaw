import ReactMarkdown from "react-markdown";

import { Button } from "@/components/ui/button";
import { BookOpen, ChevronRight } from "@/lib/icons";
import { cn } from "@/lib/utils";

interface ExplorationPath {
  title: string;
  description: string;
  steps: string[];
}

interface QuickStartGuideProps {
  content?: string;
  explorationPaths?: ExplorationPath[];
  onPathClick?: (path: ExplorationPath) => void;
  className?: string;
}

export function QuickStartGuide({
  content,
  explorationPaths,
  onPathClick,
  className,
}: QuickStartGuideProps) {
  const hasContent =
    content || (explorationPaths && explorationPaths.length > 0);

  if (!hasContent) {
    return (
      <div className="text-center py-8 text-muted-foreground">
        <BookOpen className="h-8 w-8 mx-auto mb-2 opacity-50" />
        <p className="text-sm">No quick start guide available</p>
      </div>
    );
  }

  return (
    <div className={cn("space-y-6", className)}>
      {content && (
        <div className="prose prose-sm dark:prose-invert max-w-none">
          <ReactMarkdown>{content}</ReactMarkdown>
        </div>
      )}

      {explorationPaths && explorationPaths.length > 0 && (
        <div className="space-y-4">
          <h3 className="text-sm font-semibold">Suggested Exploration Paths</h3>
          <div className="space-y-3">
            {explorationPaths.map((path, index) => (
              <div
                key={index}
                className="rounded-lg border bg-card p-4 hover:bg-accent/50 transition-colors"
              >
                <div className="flex items-start justify-between gap-3">
                  <div className="flex-1 min-w-0">
                    <h4 className="font-medium text-sm mb-1">{path.title}</h4>
                    <p className="text-sm text-muted-foreground mb-3">
                      {path.description}
                    </p>

                    <ol className="space-y-1.5 text-sm">
                      {path.steps.map((step, stepIndex) => (
                        <li
                          key={stepIndex}
                          className="flex items-start gap-2 text-muted-foreground"
                        >
                          <span className="font-mono text-xs bg-muted px-1.5 py-0.5 rounded shrink-0">
                            {stepIndex + 1}
                          </span>
                          <span className="flex-1">{step}</span>
                        </li>
                      ))}
                    </ol>
                  </div>

                  {onPathClick && (
                    <Button
                      size="sm"
                      variant="ghost"
                      onClick={() => onPathClick(path)}
                    >
                      <ChevronRight className="h-4 w-4" />
                    </Button>
                  )}
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
