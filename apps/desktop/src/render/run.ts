import { effectiveClientLanguage } from "../view-helpers";
import type { AgentField, LoopbackPage, Project, ShellCopy, Snapshot } from "../protocol";
import { escapeHtml } from "../client-utils";
import { focusedHostIsLocal, prefillHint } from "../launch-session";
import { launchFormKey } from "../form-keys";
import { ui } from "../ui";

export function launchForm(copy: ShellCopy, snap: Snapshot): string {
  const form = snap.launchForm;
  if (!form) return "";
  if (!form.skipAgentPicker) {
    const selected = form.agents.find((agent) => agent.id === ui.launchPickerAgentId);
    const selection = selected
      ? `${copy.pickAgent}：${selected.name}`
      : copy.noAgentSelected;
    return `<div class="overlay modal" data-act="close-launch">
      <div class="sheet form-sheet launch-sheet" data-act="form-noop">
        <h2>${escapeHtml(copy.pickAgent)}</h2>
        <div class="choices agent-picks">
          ${form.agents
            .map(
              (agent) =>
                `<div class="agent-choice ${agent.installed ? "" : "agent-choice-unavailable"}">
                  <button type="button" class="${agent.id === ui.launchPickerAgentId ? "active" : ""}" aria-pressed="${agent.id === ui.launchPickerAgentId ? "true" : "false"}" data-act="select-agent" data-id="${escapeHtml(agent.id)}" ${agent.installed ? "" : "disabled"}>${escapeHtml(agent.name)}</button>
                  ${agent.installed || !agent.unavailableReason ? "" : `<p class="notice bad">${escapeHtml(agent.unavailableReason)}</p>`}
                </div>`,
            )
            .join("")}
        </div>
        <p class="hint agent-selection" aria-live="polite">${escapeHtml(selection)}</p>
        <div class="actions">
          <button type="button" data-act="close-launch">${escapeHtml(copy.cancel)}</button>
          <button type="button" class="primary" data-act="next-agent" ${selected?.installed ? "" : "disabled"}>${escapeHtml(copy.nextStep)}</button>
        </div>
      </div>
    </div>`;
  }
  if (!ui.launchDraft) return "";
  const draft = ui.launchDraft;
  const first = form.fields.filter((field) => !field.folded && field.id !== "initial-instruction");
  const folded = form.fields.filter((field) => field.folded);
  const intentActive = draft.custom ? "" : draft.intentId;
  const key = launchFormKey(form.projectId);
  const pending = ui.formOperations.pending.has(key);
  const error = ui.formOperations.errors.get(key) || form.error || "";
  return `<div class="overlay modal" data-act="close-launch">
    <form class="sheet form-sheet launch-sheet" data-act="form-noop" data-form="launch" aria-busy="${pending ? "true" : "false"}">
      <h2>${escapeHtml(copy.launchTitle)}</h2>
      <fieldset class="launch-fields" ${pending ? "disabled" : ""}>
      <div class="launch-agent">
        <b>${escapeHtml(form.agents.find((agent) => agent.id === form.selectedAgentId)?.name ?? form.selectedAgentId)}</b>
        <button type="button" data-act="switch-agent">${escapeHtml(copy.switchAgent)}</button>
      </div>
      <p class="hint">${escapeHtml(prefillHint(copy, form.prefillSource))}</p>
      <div class="field">
        <div class="label">${escapeHtml(copy.runIntent)}</div>
        <div class="choices">
          <button type="button" class="${intentActive === "" && !draft.custom ? "active" : ""}" data-act="intent" data-id="">${escapeHtml(copy.intentNone)}</button>
          ${form.intents
            .map(
              (intent) =>
                `<button type="button" class="${intentActive === intent.id ? "active" : ""}" data-act="intent" data-id="${escapeHtml(intent.id)}">${escapeHtml(intent.label)}</button>`,
            )
            .join("")}
          <button type="button" class="active" data-act="intent-custom" ${draft.custom ? "" : "hidden"}>${escapeHtml(copy.intentCustom)}</button>
        </div>
      </div>
      <div class="field">
        <label class="label" for="opening-text">${escapeHtml(copy.openingPlaceholder)}</label>
        <textarea id="opening-text" data-field="openingText" rows="4" required placeholder="${escapeHtml(copy.openingPlaceholder)}">${escapeHtml(draft.openingText)}</textarea>
      </div>
      ${first.map((field) => launchField(field, draft.values[field.id] ?? "", draft.values)).join("")}
      <div class="field">
        <div class="label">${escapeHtml(copy.workingDirectory)}</div>
        <input value="${escapeHtml(form.workingDirectory)}" readonly />
      </div>
      <label class="graph-opt ${form.isolationSupported ? "" : "isolation-off"}">
        <input type="checkbox" data-launch="isolation" ${draft.values.isolation === "true" ? "checked" : ""} ${form.isolationSupported ? "" : "disabled"} />
        ${escapeHtml(copy.isolation)}
      </label>
      <p class="hint">${escapeHtml(copy.isolationHint)}</p>
      ${
        form.isolationSupported
          ? ""
          : `<details class="isolation-why"><summary>${escapeHtml(copy.isolationOffReason)}</summary><p class="hint">${escapeHtml(form.isolationReason)}</p></details>`
      }
      <details class="folded" ${ui.launchFolded ? "open" : ""}>
        <summary data-act="toggle-folded">${escapeHtml(copy.foldedOptions)}</summary>
        ${folded.map((field) => launchField(field, draft.values[field.id] ?? "", draft.values)).join("")}
        ${
          snap.showCommandPreview
            ? `<div class="field"><div class="label">${escapeHtml(copy.commandPreview)}</div><pre class="payload launch-command-preview">${escapeHtml(form.commandPreview)}</pre></div>`
            : ""
        }
      </details>
      <p class="notice launch-warnings" ${form.warnings?.length ? "" : "hidden"}>${escapeHtml((form.warnings ?? []).join(" "))}</p>
      ${form.optionDiscoveryError ? `<p class="notice">${escapeHtml(form.optionDiscoveryError)}</p>` : ""}
      ${error ? `<p class="notice bad form-feedback">${escapeHtml(error)}</p>` : ""}
      <div class="actions">
        <button type="button" data-act="close-launch" ${pending ? "disabled" : ""}>${escapeHtml(copy.cancel)}</button>
        <button type="submit" class="primary" ${pending ? "disabled" : ""}>${escapeHtml(pending ? copy.startRunPending : copy.startRun)}</button>
      </div>
      </fieldset>
    </form>
  </div>`;
}

export function launchField(field: AgentField, value: string, values: Record<string, string>): string {
  const id = `launch-${field.id}`;
  if (field.kind === "boolean") {
    return `<label class="graph-opt">
      <input type="checkbox" data-launch="${escapeHtml(field.id)}" ${value === "true" ? "checked" : ""} />
      ${escapeHtml(field.label)}
    </label>`;
  }
  if (field.kind === "select") {
    const options = launchFieldOptions(field, values);
    const known = options.includes(value);
    const customValue = known ? "" : value;
    return `<div class="field">
      <label class="label" for="${id}">${escapeHtml(field.label)}</label>
      <select id="${id}" data-launch-select="${escapeHtml(field.id)}" data-launch="${escapeHtml(field.id)}" ${field.required ? "required" : ""}>
        ${launchSelectOptions(options, value)}
      </select>
      <input class="launch-custom-value" data-launch-custom="${escapeHtml(field.id)}" value="${escapeHtml(customValue)}" ${customValue ? "" : "hidden"} placeholder="${escapeHtml(customOptionLabel())}" ${customValue && field.required ? "required" : ""} />
    </div>`;
  }
  if (field.kind === "multiline") {
    return `<div class="field">
      <label class="label" for="${id}">${escapeHtml(field.label)}</label>
      <textarea id="${id}" data-launch="${escapeHtml(field.id)}" rows="3">${escapeHtml(value)}</textarea>
    </div>`;
  }
  return `<div class="field">
    <label class="label" for="${id}">${escapeHtml(field.label)}</label>
    <input id="${id}" data-launch="${escapeHtml(field.id)}" value="${escapeHtml(value)}" ${field.required ? "required" : ""} />
  </div>`;
}

export function customOptionLabel(): string {
  return effectiveClientLanguage() === "zh-CN" ? "自定义值" : "Custom value";
}

export function launchSelectOptions(options: string[], value: string): string {
  const selectedValue = options.includes(value) ? value : value ? "__custom__" : "";
  const placeholder = value ? "" : `<option value="" disabled selected>${escapeHtml(selectPlaceholderLabel())}</option>`;
  return `${placeholder}${options
    .map((option) => `<option value="${escapeHtml(option)}" ${option === selectedValue ? "selected" : ""}>${escapeHtml(option)}</option>`)
    .join("")}<option value="__custom__" ${selectedValue === "__custom__" ? "selected" : ""}>${escapeHtml(customOptionLabel())}</option>`;
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
  const activeRun = Boolean(
    ui.snapshot?.projects.find((project) => project.id === ui.formProjectId)?.hasActiveRun,
  );
  const lockedRegistration = editing && activeRun;
  const saving = ui.projectOperation === "save";
  const inferenceCandidate = ui.projectInference.status === "candidate" ? ui.projectInference.candidate : null;
  const inferenceMessage = ui.projectInference.status === "failed" ? ui.projectInference.message : "";
  return `<div class="overlay modal" data-act="close-form">
    <form class="sheet form-sheet" data-act="form-noop" data-form="project">
      <h2>${escapeHtml(editing ? copy.editProjectTitle : copy.registerProjectTitle)}</h2>
      <p class="hint">${escapeHtml(focusedHostIsLocal() ? copy.inferenceHint : copy.remoteProjectHint)}</p>
      ${lockedRegistration ? `<p class="notice">${escapeHtml(copy.activeProjectEditHint)}</p>` : ""}
      <div class="field">
        <label class="label" for="project-name">${escapeHtml(copy.displayName)}</label>
        <input id="project-name" data-field="name" ${saving ? "disabled" : "required"} value="${escapeHtml(ui.formDraft.name)}" />
      </div>
      <div class="field">
        <label class="label" for="project-path">${escapeHtml(copy.localDirectory)}</label>
        <div class="path-picker">
          <input id="project-path" class="path-input" data-field="localPath" ${lockedRegistration || saving ? "disabled" : "required"} value="${escapeHtml(ui.formDraft.localPath)}" title="${escapeHtml(ui.formDraft.localPath)}" dir="ltr" />
          ${focusedHostIsLocal() && !lockedRegistration
            ? `<button type="button" data-act="choose-project-directory" ${saving ? "disabled" : ""}>${escapeHtml(copy.chooseDirectory)}</button>`
            : ""}
        </div>
      </div>
      <div class="field">
        <label class="label" for="project-host">${escapeHtml(copy.githubHost)}</label>
        <input id="project-host" data-field="githubHost" ${lockedRegistration || saving ? "disabled" : ""} value="${escapeHtml(ui.formDraft.githubHost)}" />
      </div>
      <div class="field">
        <label class="label" for="project-repo">${escapeHtml(copy.repository)}</label>
        <input id="project-repo" data-field="repository" ${lockedRegistration || saving ? "disabled" : "required"} placeholder="owner/repo" value="${escapeHtml(ui.formDraft.repository)}" />
      </div>
      ${!lockedRegistration && ui.projectInference.status !== "idle"
        ? `<div class="inference">
            ${ui.projectInference.status === "pending"
              ? `<p class="hint" data-inference="pending">${escapeHtml(copy.inferringFromDirectory)}</p>`
              : ""}
            ${inferenceCandidate
              ? `<div class="notice ok inference-candidate" data-inference="candidate">
                  <div><b>${escapeHtml(inferenceCandidate.name)}</b></div>
                  <div>${escapeHtml(inferenceCandidate.githubHost)}/${escapeHtml(inferenceCandidate.repository)}</div>
                  <div class="actions">
                    <button type="button" data-act="apply-infer">${escapeHtml(copy.useInference)}</button>
                  </div>
                </div>`
              : ""}
            ${inferenceMessage
              ? `<div class="notice bad" data-inference="failed">
                  <div>${escapeHtml(inferenceMessage)}</div>
                  <div class="actions">
                    <button type="button" data-act="retry-infer">${escapeHtml(copy.retryInference)}</button>
                  </div>
                </div>`
              : ""}
          </div>`
        : ""}
      ${ui.formError ? `<p class="notice bad">${escapeHtml(ui.formError)}</p>` : ""}
      <div class="actions">
        <button type="button" data-act="close-form" ${saving ? "disabled" : ""}>${escapeHtml(copy.cancel)}</button>
        <button type="submit" class="primary" ${saving ? "disabled" : ""}>${escapeHtml(saving ? copy.operationPending : editing ? copy.saveRegistration : copy.addProject)}</button>
      </div>
    </form>
  </div>`;
}

export function removeDialog(copy: ShellCopy, project: Project): string {
  if (project.hasActiveRun) {
    return `<div class="overlay modal" data-act="close-remove">
      <div class="sheet" data-act="form-noop">
        <h2>${escapeHtml(copy.cannotRemoveActiveRun)} ${escapeHtml(project.name)}</h2>
        <p class="notice bad">${escapeHtml(copy.cannotRemoveActiveRunBody)}</p>
        <div class="actions">
          <button type="button" class="primary" data-act="close-remove">${escapeHtml(copy.gotIt)}</button>
        </div>
      </div>
    </div>`;
  }
  return `<div class="overlay modal" data-act="close-remove">
    <div class="sheet" data-act="form-noop">
      <h2>${escapeHtml(copy.removeConfirmTitle)}</h2>
      <p class="notice">${escapeHtml(copy.removeConfirmBody)}</p>
      ${project.hasExecutionStopped ? `<p class="notice">${escapeHtml(copy.removeKeepClaimsBody)}</p>` : ""}
      ${ui.removeError ? `<p class="notice bad">${escapeHtml(ui.removeError)}</p>` : ""}
      <div class="actions">
        <button type="button" data-act="close-remove" ${ui.projectOperation === "remove" ? "disabled" : ""}>${escapeHtml(copy.cancel)}</button>
        <button type="button" class="danger primary" data-act="confirm-remove" ${ui.projectOperation === "remove" ? "disabled" : ""}>${escapeHtml(ui.projectOperation === "remove" ? copy.removalPending : copy.removeConfirm)}</button>
      </div>
    </div>
  </div>`;
}

export function loopbackNotice(page: LoopbackPage): string {
  if (page.status === "serving") return "";
  return `<p class="notice">${escapeHtml(page.reason)}</p>`;
}
