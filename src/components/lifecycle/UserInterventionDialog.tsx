/**
 * UserInterventionDialog Component
 *
 * Modal dialog for handling Tier 3 escalation events where user
 * intervention is required to proceed with recovery.
 */

import { useEffect } from "react";
import { useLifecycleStore } from "@/stores/lifecycleStore";
import { cn } from "@/lib/utils";
import {
  AlertTriangle,
  AlertCircle,
  RefreshCw,
  XCircle,
  Settings,
  Info,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Separator } from "@/components/ui/separator";
import type { UserInterventionRequest, InterventionOption } from "@/stores/lifecycleStore";

interface UserInterventionDialogProps {
  className?: string;
  /** Optional intervention to show (if not using global store) */
  intervention?: UserInterventionRequest | null;
  /** Called when intervention is resolved */
  onResolve?: (requestId: string, optionId: string) => void;
  /** Called when dialog is dismissed */
  onDismiss?: () => void;
}

export function UserInterventionDialog({
  className,
  intervention: propIntervention,
  onResolve,
  onDismiss,
}: UserInterventionDialogProps) {
  const {
    activeIntervention,
    setActiveIntervention,
    resolveIntervention,
    isLoading,
  } = useLifecycleStore();

  // Use prop intervention or store intervention
  const intervention = propIntervention ?? activeIntervention;
  const isOpen = intervention !== null;

  // Auto-show dialog when there's an active intervention
  useEffect(() => {
    if (activeIntervention && !propIntervention) {
      // Could trigger a notification here
    }
  }, [activeIntervention, propIntervention]);

  const handleSelectOption = async (option: InterventionOption) => {
    if (!intervention) return;

    if (option.destructive) {
      const confirmed = window.confirm(
        `"${option.label}" is a destructive action. Are you sure you want to proceed?`
      );
      if (!confirmed) return;
    }

    if (onResolve) {
      onResolve(intervention.id, option.id);
    } else {
      await resolveIntervention(intervention.id, option.id);
    }
  };

  const handleDismiss = () => {
    if (onDismiss) {
      onDismiss();
    } else {
      setActiveIntervention(null);
    }
  };

  const getOptionIcon = (optionId: string) => {
    switch (optionId.toLowerCase()) {
      case "retry":
        return <RefreshCw className="h-5 w-5" />;
      case "abort":
        return <XCircle className="h-5 w-5" />;
      case "change_config":
        return <Settings className="h-5 w-5" />;
      default:
        return <Info className="h-5 w-5" />;
    }
  };

  const formatDuration = (seconds: number) => {
    if (seconds < 60) return `${seconds}s`;
    if (seconds < 3600) return `${Math.floor(seconds / 60)}m`;
    return `${Math.floor(seconds / 3600)}h ${Math.floor((seconds % 3600) / 60)}m`;
  };

  if (!intervention) return null;

  return (
    <Dialog open={isOpen} onOpenChange={(open) => !open && handleDismiss()}>
      <DialogContent className={cn("sm:max-w-lg", className)}>
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2 text-red-600">
            <AlertTriangle className="h-5 w-5" />
            User Intervention Required
          </DialogTitle>
          <DialogDescription>
            Automated recovery has been exhausted. Please choose how to proceed.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4">
          {/* Failure Context */}
          <Card className="border-red-200 bg-red-50 dark:border-red-900 dark:bg-red-950">
            <CardHeader className="pb-2">
              <CardTitle className="text-sm flex items-center gap-2 text-red-700 dark:text-red-400">
                <AlertCircle className="h-4 w-4" />
                Failure Details
              </CardTitle>
            </CardHeader>
            <CardContent className="text-sm space-y-2">
              <div className="grid grid-cols-2 gap-2">
                <div>
                  <span className="text-muted-foreground">Resource:</span>
                  <p className="font-mono text-xs truncate">
                    {intervention.resourceId}
                  </p>
                </div>
                <div>
                  <span className="text-muted-foreground">Type:</span>
                  <p>{intervention.resourceType}</p>
                </div>
              </div>

              <Separator className="my-2" />

              <div>
                <span className="text-muted-foreground">Error:</span>
                <p className="text-red-700 dark:text-red-400">
                  {intervention.failureContext.error}
                </p>
              </div>

              <div className="grid grid-cols-2 gap-2 text-xs text-muted-foreground">
                <div>
                  Recovery attempts: {intervention.failureContext.recoveryAttempts}
                </div>
                <div>
                  Running duration:{" "}
                  {formatDuration(intervention.failureContext.runningDurationSecs)}
                </div>
              </div>

              {intervention.attemptedTiers.length > 0 && (
                <div className="flex items-center gap-1 flex-wrap">
                  <span className="text-xs text-muted-foreground">
                    Tiers attempted:
                  </span>
                  {intervention.attemptedTiers.map((tier) => (
                    <Badge key={tier} variant="secondary" className="text-xs">
                      Tier {tier}
                    </Badge>
                  ))}
                </div>
              )}
            </CardContent>
          </Card>

          {/* Options */}
          <div className="space-y-2">
            <h4 className="text-sm font-medium">Choose an action:</h4>
            <div className="grid gap-2">
              {intervention.options.map((option) => (
                <Button
                  key={option.id}
                  variant={option.destructive ? "destructive" : "outline"}
                  className={cn(
                    "justify-start h-auto py-3 px-4",
                    !option.destructive &&
                      "hover:bg-green-50 hover:border-green-300 dark:hover:bg-green-950"
                  )}
                  onClick={() => handleSelectOption(option)}
                  disabled={isLoading}
                >
                  <div className="flex items-start gap-3">
                    <div
                      className={cn(
                        "mt-0.5",
                        option.destructive
                          ? "text-red-500"
                          : "text-green-600 dark:text-green-400"
                      )}
                    >
                      {getOptionIcon(option.id)}
                    </div>
                    <div className="text-left">
                      <div className="font-medium">{option.label}</div>
                      <div className="text-xs text-muted-foreground">
                        {option.description}
                      </div>
                    </div>
                  </div>
                </Button>
              ))}
            </div>
          </div>
        </div>

        <DialogFooter className="gap-2 sm:gap-0">
          <Button variant="ghost" onClick={handleDismiss} disabled={isLoading}>
            Dismiss (decide later)
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

export default UserInterventionDialog;
