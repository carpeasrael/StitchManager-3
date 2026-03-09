import * as SettingsService from "../services/SettingsService";
import * as AiService from "../services/AiService";

export class SettingsDialog {
  private overlay: HTMLElement | null = null;

  static async open(): Promise<void> {
    const dialog = new SettingsDialog();
    await dialog.show();
  }

  private async show(): Promise<void> {
    // Load current settings
    const settings = await SettingsService.getAllSettings();

    this.overlay = document.createElement("div");
    this.overlay.className = "dialog-overlay";
    this.overlay.addEventListener("click", (e) => {
      if (e.target === this.overlay) this.close();
    });

    const dialog = document.createElement("div");
    dialog.className = "dialog dialog-settings";

    // Header
    const header = document.createElement("div");
    header.className = "dialog-header";
    header.innerHTML =
      '<span class="dialog-title">Einstellungen</span>';
    const closeBtn = document.createElement("button");
    closeBtn.className = "dialog-close";
    closeBtn.textContent = "\u00D7";
    closeBtn.addEventListener("click", () => this.close());
    header.appendChild(closeBtn);
    dialog.appendChild(header);

    // Body
    const body = document.createElement("div");
    body.className = "dialog-body";

    // Tab bar
    const tabBar = document.createElement("div");
    tabBar.className = "dialog-tab-bar";

    const aiTab = document.createElement("button");
    aiTab.className = "dialog-tab active";
    aiTab.textContent = "KI-Einstellungen";
    aiTab.dataset.tab = "ki";
    tabBar.appendChild(aiTab);

    const fileTab = document.createElement("button");
    fileTab.className = "dialog-tab";
    fileTab.textContent = "Dateiverwaltung";
    fileTab.dataset.tab = "files";
    tabBar.appendChild(fileTab);

    body.appendChild(tabBar);

    // KI settings form
    const kiForm = document.createElement("div");
    kiForm.className = "settings-form settings-tab-content";
    kiForm.dataset.tabContent = "ki";
    this.buildKiTab(kiForm, settings);
    body.appendChild(kiForm);

    // Dateiverwaltung form
    const filesForm = document.createElement("div");
    filesForm.className = "settings-form settings-tab-content";
    filesForm.dataset.tabContent = "files";
    filesForm.style.display = "none";
    this.buildFilesTab(filesForm, settings);
    body.appendChild(filesForm);

    // Tab switching
    const tabs = tabBar.querySelectorAll<HTMLButtonElement>(".dialog-tab");
    tabs.forEach((tab) => {
      tab.addEventListener("click", () => {
        tabs.forEach((t) => t.classList.remove("active"));
        tab.classList.add("active");
        const tabName = tab.dataset.tab;
        body.querySelectorAll<HTMLElement>(".settings-tab-content").forEach((c) => {
          c.style.display = c.dataset.tabContent === tabName ? "" : "none";
        });
      });
    });

    dialog.appendChild(body);

    // Footer
    const footer = document.createElement("div");
    footer.className = "dialog-footer";

    const cancelBtn = document.createElement("button");
    cancelBtn.className = "dialog-btn dialog-btn-secondary";
    cancelBtn.textContent = "Abbrechen";
    cancelBtn.addEventListener("click", () => this.close());
    footer.appendChild(cancelBtn);

    const saveBtn = document.createElement("button");
    saveBtn.className = "dialog-btn dialog-btn-primary";
    saveBtn.textContent = "Speichern";
    saveBtn.addEventListener("click", async () => {
      saveBtn.disabled = true;
      saveBtn.textContent = "Speichere...";
      // Save both tabs
      await this.saveSettings(kiForm);
      await this.saveSettings(filesForm);
      this.close();
    });
    footer.appendChild(saveBtn);

    dialog.appendChild(footer);
    this.overlay.appendChild(dialog);
    document.body.appendChild(this.overlay);
  }

  private buildKiTab(
    form: HTMLElement,
    settings: Record<string, string>
  ): void {
    // Provider
    const providerGroup = this.createFormGroup("Provider");
    const providerSelect = document.createElement("select");
    providerSelect.className = "settings-input";
    providerSelect.dataset.key = "ai_provider";

    for (const opt of ["ollama", "openai"]) {
      const option = document.createElement("option");
      option.value = opt;
      option.textContent = opt === "ollama" ? "Ollama" : "OpenAI";
      if (settings.ai_provider === opt) option.selected = true;
      providerSelect.appendChild(option);
    }
    providerGroup.appendChild(providerSelect);
    form.appendChild(providerGroup);

    // URL
    const urlGroup = this.createFormGroup("URL");
    const urlInput = document.createElement("input");
    urlInput.type = "text";
    urlInput.className = "settings-input";
    urlInput.dataset.key = "ai_url";
    urlInput.value = settings.ai_url || "http://localhost:11434";
    urlInput.placeholder = "http://localhost:11434";
    urlGroup.appendChild(urlInput);
    form.appendChild(urlGroup);

    // API Key (only visible for OpenAI)
    const apiKeyGroup = this.createFormGroup("API-Schluessel");
    apiKeyGroup.className = "settings-form-group settings-api-key-group";
    const apiKeyInput = document.createElement("input");
    apiKeyInput.type = "password";
    apiKeyInput.className = "settings-input";
    apiKeyInput.dataset.key = "ai_api_key";
    apiKeyInput.value = settings.ai_api_key || "";
    apiKeyInput.placeholder = "sk-...";
    apiKeyGroup.appendChild(apiKeyInput);
    form.appendChild(apiKeyGroup);

    // Toggle API key visibility
    const updateApiKeyVisibility = () => {
      apiKeyGroup.style.display =
        providerSelect.value === "openai" ? "" : "none";
    };
    providerSelect.addEventListener("change", updateApiKeyVisibility);
    updateApiKeyVisibility();

    // Model
    const modelGroup = this.createFormGroup("Modell");
    const modelInput = document.createElement("input");
    modelInput.type = "text";
    modelInput.className = "settings-input";
    modelInput.dataset.key = "ai_model";
    modelInput.value = settings.ai_model || "llama3.2-vision";
    modelGroup.appendChild(modelInput);
    form.appendChild(modelGroup);

    // Temperature
    const tempGroup = this.createFormGroup("Temperatur");
    const tempWrapper = document.createElement("div");
    tempWrapper.className = "settings-range-wrapper";

    const tempSlider = document.createElement("input");
    tempSlider.type = "range";
    tempSlider.className = "settings-range";
    tempSlider.min = "0";
    tempSlider.max = "1";
    tempSlider.step = "0.1";
    tempSlider.dataset.key = "ai_temperature";
    tempSlider.value = settings.ai_temperature || "0.3";

    const tempDisplay = document.createElement("span");
    tempDisplay.className = "settings-range-value";
    tempDisplay.textContent = tempSlider.value;
    tempSlider.addEventListener("input", () => {
      tempDisplay.textContent = tempSlider.value;
    });

    tempWrapper.appendChild(tempSlider);
    tempWrapper.appendChild(tempDisplay);
    tempGroup.appendChild(tempWrapper);
    form.appendChild(tempGroup);

    // Timeout
    const timeoutGroup = this.createFormGroup("Timeout (ms)");
    const timeoutInput = document.createElement("input");
    timeoutInput.type = "number";
    timeoutInput.className = "settings-input";
    timeoutInput.dataset.key = "ai_timeout_ms";
    timeoutInput.min = "5000";
    timeoutInput.max = "120000";
    timeoutInput.step = "1000";
    timeoutInput.value = settings.ai_timeout_ms || "30000";
    timeoutGroup.appendChild(timeoutInput);
    form.appendChild(timeoutGroup);

    // Test connection button
    const testGroup = document.createElement("div");
    testGroup.className = "settings-form-group settings-test-group";

    const testBtn = document.createElement("button");
    testBtn.className = "dialog-btn dialog-btn-secondary";
    testBtn.textContent = "Verbindung testen";

    const testStatus = document.createElement("span");
    testStatus.className = "settings-test-status";

    testBtn.addEventListener("click", async () => {
      testBtn.disabled = true;
      testStatus.textContent = "Teste...";
      testStatus.className = "settings-test-status";

      // Save settings first so the test uses current values
      await this.saveSettings(form);

      try {
        const ok = await AiService.testConnection();
        if (ok) {
          testStatus.textContent = "Verbindung erfolgreich";
          testStatus.className = "settings-test-status settings-test-ok";
        } else {
          testStatus.textContent = "Verbindung fehlgeschlagen";
          testStatus.className = "settings-test-status settings-test-fail";
        }
      } catch {
        testStatus.textContent = "Fehler beim Test";
        testStatus.className = "settings-test-status settings-test-fail";
      }
      testBtn.disabled = false;
    });

    testGroup.appendChild(testBtn);
    testGroup.appendChild(testStatus);
    form.appendChild(testGroup);
  }

  private buildFilesTab(
    form: HTMLElement,
    settings: Record<string, string>
  ): void {
    // Platzhalter-Legende
    const legend = document.createElement("div");
    legend.className = "settings-legend";
    legend.innerHTML =
      "<strong>Platzhalter:</strong> " +
      "<code>{name}</code> Anzeigename, " +
      "<code>{theme}</code> Thema, " +
      "<code>{format}</code> Dateiformat";
    form.appendChild(legend);

    // Umbennungsmuster
    const renameGroup = this.createFormGroup("Umbennungsmuster");
    const renameInput = document.createElement("input");
    renameInput.type = "text";
    renameInput.className = "settings-input";
    renameInput.dataset.key = "rename_pattern";
    renameInput.value = settings.rename_pattern || "{name}_{theme}";
    renameInput.placeholder = "{name}_{theme}";
    renameGroup.appendChild(renameInput);
    form.appendChild(renameGroup);

    // Organisationsmuster
    const organizeGroup = this.createFormGroup("Organisationsmuster");
    const organizeInput = document.createElement("input");
    organizeInput.type = "text";
    organizeInput.className = "settings-input";
    organizeInput.dataset.key = "organize_pattern";
    organizeInput.value = settings.organize_pattern || "{theme}/{name}";
    organizeInput.placeholder = "{theme}/{name}";
    organizeGroup.appendChild(organizeInput);
    form.appendChild(organizeGroup);

    // Bibliotheks-Stammverzeichnis
    const libraryGroup = this.createFormGroup("Bibliotheks-Stammverzeichnis");
    const libraryInput = document.createElement("input");
    libraryInput.type = "text";
    libraryInput.className = "settings-input";
    libraryInput.dataset.key = "library_root";
    libraryInput.value = settings.library_root || "~/Stickdateien";
    libraryInput.placeholder = "~/Stickdateien";
    libraryGroup.appendChild(libraryInput);
    form.appendChild(libraryGroup);

    // Metadaten-Verzeichnis
    const metaGroup = this.createFormGroup("Metadaten-Verzeichnis");
    const metaInput = document.createElement("input");
    metaInput.type = "text";
    metaInput.className = "settings-input";
    metaInput.dataset.key = "metadata_root";
    metaInput.value = settings.metadata_root || "~/Stickdateien/.stichman";
    metaInput.placeholder = "~/Stickdateien/.stichman";
    metaGroup.appendChild(metaInput);
    form.appendChild(metaGroup);
  }

  private createFormGroup(label: string): HTMLElement {
    const group = document.createElement("div");
    group.className = "settings-form-group";

    const labelEl = document.createElement("label");
    labelEl.className = "settings-label";
    labelEl.textContent = label;
    group.appendChild(labelEl);

    return group;
  }

  private async saveSettings(form: HTMLElement): Promise<boolean> {
    const inputs = form.querySelectorAll<
      HTMLInputElement | HTMLSelectElement
    >("[data-key]");

    let allOk = true;
    for (const input of inputs) {
      const key = input.dataset.key;
      if (!key) continue;

      // Skip API key if it's empty and provider isn't openai
      if (key === "ai_api_key") {
        const provider = form.querySelector<HTMLSelectElement>(
          '[data-key="ai_provider"]'
        );
        if (provider && provider.value !== "openai") continue;
        if (!input.value) continue;
      }

      try {
        await SettingsService.setSetting(key, input.value);
      } catch (e) {
        console.warn(`Failed to save setting '${key}':`, e);
        allOk = false;
      }
    }
    return allOk;
  }

  private close(): void {
    if (this.overlay) {
      this.overlay.remove();
      this.overlay = null;
    }
  }
}
