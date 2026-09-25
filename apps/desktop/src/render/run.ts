import { effectiveClientLanguage, mobileClient } from "../view-helpers";
import type { AgentField, LoopbackPage, Project, ShellCopy, Snapshot } from "../protocol";
import { escapeHtml } from "../client-utils";
import { focusedHostIsLocal, prefillHint } from "../launch-session";
import { launchFormKey } from "../form-keys";
import { startupCopy } from "../startup-copy";
import { confirmationDialog, dialog, dialogActionButton, dialogDismissButton } from "../components/dialog";
import { button, checkbox, formField, notice, textArea, textInput } from "../components/primitives";
import { ui } from "../ui";
import { runPersistenceWritesBlocked } from "./run-organization";

export function launchForm(copy: ShellCopy, snap: Snapshot): string {
  const form = snap.launchForm;
  if (!form) return "";
  const localCopy = startupCopy(effectiveClientLanguage());
  const persistenceBlocked = !mobileClient() && runPersistenceWritesBlocked(snap);
  if (!form.skipAgentPicker) {
    const selected = form.agents.find((agent) => agent.id === ui.launchPickerAgentId);
    const selection = selected ? `${copy.pickAgent}：${selected.name}` : copy.noAgentSelected;
    const availableAgents = form.agents.filter((agent) => agent.installed);
    const unavailableAgents = form.agents.filter((agent) => !agent.installed);
    const renderAgent = (agent: typeof form.agents[number]) => {
      const reasonId = `agent-reason-${agent.id}`;
      return `<div class="agent-choice ${agent.installed ? "" : "agent-choice-unavailable"}">
        ${button({
          id: "select-agent",
          label: agent.name,
          disabled: !agent.installed,
          pressed: agent.id === ui.launchPickerAgentId,
          data: { id: agent.id },
        }, {
          variant: "secondary",
          className: "agent-choice-button",
          attributes: {
            "data-agent-mark": agent.name.slice(0, 1).toUpperCase(),
            "aria-describedby": !agent.installed && agent.unavailableReason ? reasonId : undefined,
          },
        })}
        ${agent.installed || !agent.unavailableReason
          ? ""
          : `<p id="${escapeHtml(reasonId)}" class="agent-choice-reason">${escapeHtml(agent.unavailableReason)}</p>`}
      </div>`;
    };
    const body = `<div class="agent-picker">
      ${availableAgents.length > 0 ? `<section class="agent-picker-group" aria-labelledby="available-agents-title">
        <h3 id="available-agents-title">${escapeHtml(copy.availableAgents)}</h3>
        <div class="agent-picks">${availableAgents.map(renderAgent).join("")}</div>
      </section>` : ""}
      ${unavailableAgents.length > 0 ? `<section class="agent-picker-group agent-picker-unavailable" aria-labelledby="unavailable-agents-title">
        <h3 id="unavailable-agents-title">${escapeHtml(copy.unavailableAgents)}</h3>
        <div class="agent-picks">${unavailableAgents.map(renderAgent).join("")}</div>
      </section>` : ""}
    </div>`;
    const actions = `<div class="agent-selection" aria-live="polite">${escapeHtml(selection)}</div>
      ${dialogDismissButton(copy.cancel)}${dialogActionButton({
      id: "next-agent",
      label: copy.nextStep,
      disabled: !selected?.installed,
    }, { primary: true })}`;
    return dialog({
      id: "launch",
      tier: "form",
      title: copy.pickAgent,
      body,
      actions,
      closeLabel: localCopy.close,
      dismissible: true,
      initialFocus: "first-field",
      className: "launch-sheet launch-agent-picker",
    });
  }
  if (!ui.launchDraft) return "";
  const draft = ui.launchDraft;
  const first = form.fields.filter((field) => !field.folded && field.id !== "initial-instruction");
  const folded = form.fields.filter((field) => field.folded);
  const key = launchFormKey(form.projectId);
  const pending = ui.formOperations.pending.has(key);
  const error = ui.formOperations.errors.get(key) || form.error || "";
  const body = `<fieldset class="launch-fields" ${pending ? "disabled" : ""}>
    <div class="launch-agent">
      <b>${escapeHtml(form.agents.find((agent) => agent.id === form.selectedAgentId)?.name ?? form.selectedAgentId)}</b>
      ${button({ id: "switch-agent", label: copy.switchAgent })}
    </div>
    <p class="hint">${escapeHtml(prefillHint(copy, form.prefillSource))}</p>
    ${formField({
      id: "opening-text",
      label: copy.openingPlaceholder,
      required: true,
      control: textArea({
        id: "opening-text",
        value: draft.openingText,
        rows: 4,
        required: true,
        placeholder: copy.openingPlaceholder,
        attributes: { "data-field": "openingText" },
      }),
    })}
    ${first.map((field) => launchField(field, draft.values[field.id] ?? "", draft.values, form.optionDiscoveryPending)).join("")}
    ${formField({
      label: copy.workingDirectory,
      control: textInput({ value: form.workingDirectory, readonly: true }),
    })}
    ${checkbox({
      label: copy.isolation,
      checked: draft.values.isolation === "true",
      disabled: !form.isolationSupported,
      className: form.isolationSupported ? "" : "isolation-off",
      attributes: { "data-launch": "isolation" },
    })}
    <p class="hint">${escapeHtml(copy.isolationHint)}</p>
    ${form.isolationSupported ? "" : `<details class="isolation-why"><summary>${escapeHtml(copy.isolationOffReason)}</summary><p class="hint">${escapeHtml(form.isolationReason)}</p></details>`}
    <details class="folded" ${ui.launchFolded ? "open" : ""}>
      <summary data-act="toggle-folded">${escapeHtml(copy.foldedOptions)}</summary>
      ${folded.map((field) => launchField(field, draft.values[field.id] ?? "", draft.values, form.optionDiscoveryPending)).join("")}
      ${snap.showCommandPreview ? formField({ label: copy.commandPreview, control: `<pre class="payload launch-command-preview">${escapeHtml(form.commandPreview)}</pre>` }) : ""}
    </details>
    ${notice({ status: "warning", className: "launch-warnings", message: (form.warnings ?? []).join(" "), attributes: { hidden: !form.warnings?.length } })}
    ${form.optionDiscoveryError ? notice({ message: form.optionDiscoveryError }) : ""}
    ${persistenceBlocked ? notice({ status: "danger", role: "alert", message: copy.runPersistenceWriteBlocked }) : ""}
    ${error ? notice({ status: "danger", role: "alert", className: "form-feedback", message: error }) : ""}
  </fieldset>`;
  const actions = `${dialogDismissButton(copy.cancel, { disabled: pending })}${dialogActionButton({
    id: "submit-launch",
    label: pending ? copy.startRunPending : copy.startRun,
    disabled: pending || persistenceBlocked,
    busy: pending,
  }, { primary: true, type: "submit" })}`;
  return dialog({
    id: "launch",
    tier: "wide",
    title: copy.launchTitle,
    body,
    actions,
    closeLabel: localCopy.close,
    dismissible: !pending,
    initialFocus: "first-field",
    busy: false,
    className: "launch-sheet",
    panelTag: "form",
    panelAttributes: {
      "data-form": "launch",
      "aria-busy": pending ? "true" : "false",
      novalidate: true,
    },
  });
}

function fieldAwaitsDiscovery(field: AgentField, values: Record<string, string>, discoveryPending?: string | null): boolean {
  if (!discoveryPending) return false;
  if (field.id === "model") return true;
  return field.kind === "select" && launchFieldOptions(field, values).length === 0;
}

export function launchField(
  field: AgentField,
  value: string,
  values: Record<string, string>,
  discoveryPending?: string | null,
): string {
  const id = `launch-${field.id}`;
  const awaiting = fieldAwaitsDiscovery(field, values, discoveryPending);
  const pendingHint = awaiting && field.id === "model" && discoveryPending ? discoveryPending : undefined;
  if (field.kind === "boolean") {
    const control = checkbox({
      label: field.label,
      checked: value === "true",
      attributes: { "data-launch": field.id },
    });
    return `${control}${field.description ? `<p class="hint">${escapeHtml(field.description)}</p>` : ""}`;
  }
  const busy = awaiting ? ` data-launch-discovery="pending" aria-busy="true"` : "";
  const options = field.kind === "select" ? launchFieldOptions(field, values) : [];
  if (field.kind === "select" && options.length > 0) {
    const { customValue, customEntry } = launchSelectState(options, value);
    const customHidden = !customValue && !customEntry;
    return `<div class="ui-form-field field"${busy}>
      <label class="label" for="${id}">${escapeHtml(field.label)}</label>
      <select id="${id}" class="ui-select" data-launch-select="${escapeHtml(field.id)}" data-launch="${escapeHtml(field.id)}" ${field.required ? "required" : ""}>
        ${launchSelectOptions(options, value, field.optionLabels)}
      </select>
      ${textInput({
        value: customValue,
        required: !customHidden && field.required,
        placeholder: customOptionLabel(),
        className: "launch-custom-value",
        attributes: { "data-launch-custom": field.id, hidden: customHidden },
      })}
      ${field.description ? `<p class="hint">${escapeHtml(field.description)}</p>` : ""}
      ${pendingHint ? `<p class="hint" data-launch-discovery="pending">${escapeHtml(pendingHint)}</p>` : ""}
    </div>`;
  }
  const hint = [field.description, pendingHint].filter(Boolean).join(" ");
  const control = field.kind === "multiline"
    ? textArea({ id, value, rows: 3, attributes: { "data-launch": field.id } })
    : textInput({ id, value, required: field.required, attributes: { "data-launch": field.id } });
  return `<div${busy}>${formField({ id, label: field.label, required: field.required, hint, control })}</div>`;
}

export const CUSTOM_VALUE = "__custom__";

export function customOptionLabel(): string {
  return effectiveClientLanguage() === "zh-CN" ? "自定义值" : "Custom value";
}

export function currentValueLabel(): string {
  return effectiveClientLanguage() === "zh-CN" ? "当前值" : "current value";
}

export function launchSelectState(options: string[], value: string): { customValue: string; customEntry: boolean } {
  const customEntry = value === CUSTOM_VALUE;
  const known = options.includes(value);
  return { customValue: known || customEntry ? "" : value, customEntry };
}

export function launchSelectOptions(
  options: string[],
  value: string,
  labels: Record<string, string> = {},
): string {
  const customEntry = value === CUSTOM_VALUE;
  const known = options.includes(value);
  const placeholder = value ? "" : `<option value="" disabled selected>${escapeHtml(selectPlaceholderLabel())}</option>`;
  const current = value && !customEntry && !known
    ? `<option value="${escapeHtml(value)}" selected>${escapeHtml(value)} · ${escapeHtml(currentValueLabel())}</option>`
    : "";
  return `${placeholder}${current}${options.map((option) => {
    const label = labels[option] ? `${option} · ${labels[option]}` : option;
    return `<option value="${escapeHtml(option)}" ${option === value ? "selected" : ""}>${escapeHtml(label)}</option>`;
  }).join("")}<option value="${CUSTOM_VALUE}" ${customEntry ? "selected" : ""}>${escapeHtml(customOptionLabel())}</option>`;
}

export function selectPlaceholderLabel(): string {
  return effectiveClientLanguage() === "zh-CN" ? "请选择" : "Select a value";
}

export function launchFieldOptions(field: AgentField, values: Record<string, string>): string[] {
  const filter = field.optionFilter;
  if (!filter) return field.options ?? [];
  return filter.optionsByValue[values[filter.fieldId] ?? ""] ?? field.options ?? [];
}

export function launchFieldDefault(field: AgentField, values: Record<string, string>): string | undefined {
  const filter = field.optionFilter;
  if (!filter) return undefined;
  const options = launchFieldOptions(field, values);
  const value = filter.defaultsByValue?.[values[filter.fieldId] ?? ""];
  return value && options.includes(value) ? value : undefined;
}

export function projectForm(copy: ShellCopy): string {
  const editing = ui.formOpen === "edit";
  const activeRun = Boolean(ui.snapshot?.projects.find((project) => project.id === ui.formProjectId)?.hasActiveRun);
  const lockedRegistration = editing && activeRun;
  const saving = ui.projectOperation === "save";
  const inferenceCandidate = ui.projectInference.status === "candidate" ? ui.projectInference.candidate : null;
  const inferenceMessage = ui.projectInference.status === "failed" ? ui.projectInference.message : "";
  const localCopy = startupCopy(effectiveClientLanguage());
  const body = `<p class="hint">${escapeHtml(focusedHostIsLocal() ? copy.inferenceHint : copy.remoteProjectHint)}</p>
    ${lockedRegistration ? notice({ message: copy.activeProjectEditHint }) : ""}
    ${formField({
      id: "project-name",
      label: copy.displayName,
      required: true,
      control: textInput({ id: "project-name", value: ui.formDraft.name, required: !saving, disabled: saving, attributes: { "data-field": "name" } }),
    })}
    ${formField({
      id: "project-path",
      label: copy.localDirectory,
      required: true,
      control: `<div class="path-picker">${textInput({
        id: "project-path",
        value: ui.formDraft.localPath,
        required: !lockedRegistration && !saving,
        disabled: lockedRegistration || saving,
        className: "path-input",
        attributes: { "data-field": "localPath", title: ui.formDraft.localPath, dir: "ltr" },
      })}${focusedHostIsLocal() && !lockedRegistration ? button({ id: "choose-project-directory", label: copy.chooseDirectory, disabled: saving }) : ""}</div>`,
    })}
    ${formField({
      id: "project-host",
      label: copy.githubHost,
      control: textInput({ id: "project-host", value: ui.formDraft.githubHost, disabled: lockedRegistration || saving, attributes: { "data-field": "githubHost" } }),
    })}
    ${formField({
      id: "project-repo",
      label: copy.repository,
      required: true,
      control: textInput({ id: "project-repo", value: ui.formDraft.repository, required: !lockedRegistration && !saving, disabled: lockedRegistration || saving, placeholder: "owner/repo", attributes: { "data-field": "repository" } }),
    })}
    ${!lockedRegistration && ui.projectInference.status !== "idle" ? `<div class="inference">
      ${ui.projectInference.status === "pending" ? `<p class="hint" data-inference="pending">${escapeHtml(copy.inferringFromDirectory)}</p>` : ""}
      ${inferenceCandidate ? notice({
        status: "success",
        className: "inference-candidate",
        message: `${inferenceCandidate.name} · ${inferenceCandidate.githubHost}/${inferenceCandidate.repository}`,
        actions: `<div class="actions">${button({ id: "apply-infer", label: copy.useInference })}</div>`,
        attributes: { "data-inference": "candidate" },
      }) : ""}
      ${inferenceMessage ? notice({
        status: "danger",
        className: "inference-failed",
        message: inferenceMessage,
        actions: `<div class="actions">${button({ id: "retry-infer", label: copy.retryInference })}</div>`,
        attributes: { "data-inference": "failed" },
      }) : ""}
    </div>` : ""}
    ${ui.formError ? notice({ status: "danger", role: "alert", message: ui.formError }) : ""}`;
  const actions = `${dialogDismissButton(copy.cancel, { disabled: saving })}${dialogActionButton({
    id: "submit-project",
    label: saving ? copy.operationPending : editing ? copy.saveRegistration : copy.addProject,
    disabled: saving,
    busy: saving,
  }, { primary: true, type: "submit" })}`;
  return dialog({
    id: "project-form",
    tier: "form",
    title: editing ? copy.editProjectTitle : copy.registerProjectTitle,
    body,
    actions,
    closeLabel: localCopy.close,
    dismissible: !saving,
    initialFocus: "first-field",
    panelTag: "form",
    panelAttributes: { "data-form": "project", "aria-busy": saving ? "true" : "false" },
  });
}

export function removeDialog(copy: ShellCopy, project: Project): string {
  const localCopy = startupCopy(effectiveClientLanguage());
  const activeRunCount = project.activeRunCount;
  const count = localCopy.activeRunCount.replace("{count}", String(activeRunCount));
  const body = `${notice({ status: project.hasActiveRun ? "danger" : "warning", message: project.hasActiveRun ? localCopy.removeProjectBlocked : copy.removeConfirmBody })}
    <p class="hint">${escapeHtml(count)}</p>
    ${project.hasExecutionStopped ? notice({ message: copy.removeKeepClaimsBody }) : ""}`;
  return confirmationDialog({
    id: "remove-project",
    title: project.hasActiveRun ? `${copy.cannotRemoveActiveRun} ${project.name}` : copy.removeConfirmTitle,
    body,
    closeLabel: localCopy.close,
    cancelLabel: project.hasActiveRun ? copy.gotIt : copy.cancel,
    confirm: project.hasActiveRun ? undefined : {
      id: "confirm-remove",
      label: ui.projectOperation === "remove" ? copy.removalPending : copy.removeConfirm,
      destructive: true,
      disabled: ui.projectOperation === "remove",
      busy: ui.projectOperation === "remove",
    },
    error: ui.removeError,
  });
}

export function loopbackNotice(page: LoopbackPage): string {
  if (page.status === "serving") return "";
  return notice({ message: page.reason });
}
