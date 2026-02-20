/**
 * ApprovalOverlay — modal dialog shown when the agent needs human approval
 * to execute a tool.
 *
 * Displays: tool name, description, risk level badge, and three action buttons:
 *   • Allow Once    — approve this single execution
 *   • Always Allow  — approve and persist the policy (allow_always=true)
 *   • Deny          — reject the request
 *
 * On mobile (< 768 px) the content is rendered inside a swipe-to-dismiss
 * BottomSheet instead of a centred Dialog.
 */

import { Badge } from "@/components/ui/badge";
import { BottomSheet } from "@/components/ui/bottom-sheet";
import { Button } from "@/components/ui/button";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import type { ApprovalRequest } from "@/stores/agentStore";
import { useAgentStore } from "@/stores/agentStore";

// ─── RiskBadge ────────────────────────────────────────────────────────────────

const RISK_VARIANT: Record<
	string,
	"destructive" | "secondary" | "outline" | "default"
> = {
	high: "destructive",
	medium: "secondary",
	low: "outline",
};

function RiskBadge({ level }: { level: string }) {
	const variant = RISK_VARIANT[level.toLowerCase()] ?? "default";
	return (
		<Badge variant={variant} className="uppercase tracking-wide">
			{level} risk
		</Badge>
	);
}

// ─── ApprovalContent ──────────────────────────────────────────────────────────

interface ApprovalContentProps {
	request: ApprovalRequest;
	onAllowOnce: () => void;
	onAlwaysAllow: () => void;
	onDeny: () => void;
}

/**
 * Pure presentational component — the actual approval form.
 * Reused by both the desktop Dialog and the mobile BottomSheet.
 */
function ApprovalContent({
	request,
	onAllowOnce,
	onAlwaysAllow,
	onDeny,
}: ApprovalContentProps) {
	return (
		<div className="px-4 pb-6">
			<div className="mb-3 flex items-center gap-2">
				<span className="font-semibold text-base">Agent Approval Required</span>
				<RiskBadge level={request.riskLevel} />
			</div>

			<p className="mb-3 text-sm text-muted-foreground">
				The agent wants to run{" "}
				<code className="rounded bg-muted px-1 py-0.5 font-mono text-sm">
					{request.toolName}
				</code>
			</p>

			{request.description && (
				<p className="mb-4 text-sm text-muted-foreground leading-relaxed">
					{request.description}
				</p>
			)}

			<div className="flex flex-col gap-2 sm:flex-row">
				<Button variant="destructive" className="sm:mr-auto" onClick={onDeny}>
					Deny
				</Button>
				<Button variant="outline" onClick={onAllowOnce}>
					Allow Once
				</Button>
				<Button variant="default" onClick={onAlwaysAllow}>
					Always Allow
				</Button>
			</div>
		</div>
	);
}

// ─── SingleApprovalDialog ─────────────────────────────────────────────────────

interface SingleApprovalDialogProps {
	request: ApprovalRequest;
}

function SingleApprovalDialog({ request }: SingleApprovalDialogProps) {
	const respondToApproval = useAgentStore((s) => s.respondToApproval);

	const handleAllowOnce = () =>
		respondToApproval(request.actionId, true, false);
	const handleAlwaysAllow = () =>
		respondToApproval(request.actionId, true, true);
	const handleDeny = () => respondToApproval(request.actionId, false);

	const isMobile = typeof window !== "undefined" && window.innerWidth < 768;

	if (isMobile) {
		return (
			<BottomSheet open onClose={handleDeny}>
				<ApprovalContent
					request={request}
					onAllowOnce={handleAllowOnce}
					onAlwaysAllow={handleAlwaysAllow}
					onDeny={handleDeny}
				/>
			</BottomSheet>
		);
	}

	return (
		<Dialog open>
			<DialogContent className="max-w-md">
				<DialogHeader>
					<DialogTitle className="flex items-center gap-2">
						<span>Agent Approval Required</span>
						<RiskBadge level={request.riskLevel} />
					</DialogTitle>
					<DialogDescription>
						The agent wants to run{" "}
						<code className="rounded bg-muted px-1 py-0.5 font-mono text-sm">
							{request.toolName}
						</code>
					</DialogDescription>
				</DialogHeader>

				{request.description && (
					<p className="text-sm text-muted-foreground leading-relaxed">
						{request.description}
					</p>
				)}

				<DialogFooter className="flex flex-col gap-2 sm:flex-row">
					<Button
						variant="destructive"
						className="sm:mr-auto"
						onClick={handleDeny}
					>
						Deny
					</Button>
					<Button variant="outline" onClick={handleAllowOnce}>
						Allow Once
					</Button>
					<Button variant="default" onClick={handleAlwaysAllow}>
						Always Allow
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	);
}

// ─── ApprovalOverlay ─────────────────────────────────────────────────────────

/**
 * Mount this once near the root of the agent UI.  It renders nothing when the
 * approval queue is empty and shows the first pending request as a blocking modal.
 */
export function ApprovalOverlay() {
	const approvalQueue = useAgentStore((s) => s.approvalQueue);

	if (approvalQueue.length === 0) return null;

	// Show the oldest pending request first.
	const pending = approvalQueue[0];
	return <SingleApprovalDialog request={pending} />;
}
