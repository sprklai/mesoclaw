<script lang="ts">
	import * as m from '$lib/paraglide/messages';
	import * as AlertDialog from '$lib/components/ui/alert-dialog';
	import { Button } from '$lib/components/ui/button';

	let {
		open = $bindable(false),
		title = m.dialog_confirm_default_title(),
		description = m.dialog_confirm_default_description(),
		confirmLabel = m.dialog_confirm_default_label(),
		onConfirm
	}: {
		open: boolean;
		title?: string;
		description?: string;
		confirmLabel?: string;
		onConfirm: () => void;
	} = $props();

	function handleConfirm() {
		onConfirm();
		open = false;
	}
</script>

<AlertDialog.Root bind:open>
	<AlertDialog.Content>
		<AlertDialog.Header>
			<AlertDialog.Title>{title}</AlertDialog.Title>
			<AlertDialog.Description>{description}</AlertDialog.Description>
		</AlertDialog.Header>
		<AlertDialog.Footer>
			<AlertDialog.Cancel>{m.common_cancel()}</AlertDialog.Cancel>
			<Button variant="destructive" onclick={handleConfirm}>{confirmLabel}</Button>
		</AlertDialog.Footer>
	</AlertDialog.Content>
</AlertDialog.Root>
