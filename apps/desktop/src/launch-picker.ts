export type LaunchPickerState = {
  projectId: string;
  agentId: string;
};

export function syncLaunchPicker(
  form: { projectId: string; selectedAgentId: string },
  picker: LaunchPickerState,
): LaunchPickerState {
  if (picker.projectId !== form.projectId) {
    return { projectId: form.projectId, agentId: form.selectedAgentId };
  }
  if (!picker.agentId && form.selectedAgentId) {
    return { projectId: form.projectId, agentId: form.selectedAgentId };
  }
  return picker;
}
