import { appState } from "../state/AppState";
import { EventBus } from "../state/EventBus";
import { ToastContainer } from "./Toast";
import { trapFocus } from "../utils/focus-trap";
import * as SettingsService from "../services/SettingsService";
import * as AiService from "../services/AiService";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { applyFontSize } from "../utils/theme";
import type { ThemeMode, CustomFieldDef } from "../types/index";

export class SettingsDialog {
  private static instance: SettingsDialog | null = null;
  private overlay: HTMLElement | null = null;
  private releaseFocusTrap: (() => void) | null = null;
  private resizeObserver: ResizeObserver | null = null;
  private originalTheme: ThemeMode = "hell";
  private originalFontSize: string = "medium";
  private originalLibraryRoot: string = "";
  private originalBgImage: string = "";
  private originalBgOpacity: string = "0.15";
  private originalBgBlur: string = "0";
  private originalBgImagePath: string = "";
  private bgPathModified = false;
  private pendingBgRemove = false;

  static async open(): Promise<void> {
    if (SettingsDialog.isOpen()) return;
    const dialog = new SettingsDialog();
    SettingsDialog.instance = dialog;
    await dialog.show();
  }

  static isOpen(): boolean {
    return SettingsDialog.instance?.overlay !== null && SettingsDialog.instance?.overlay !== undefined;
  }

  static dismiss(): void {
    if (SettingsDialog.instance) {
      SettingsDialog.instance.close();
      SettingsDialog.instance = null;
    }
  }

  private async show(): Promise<void> {
    const [settings, customFields, apiKey] = await Promise.all([
      SettingsService.getAllSettings(),
      SettingsService.getCustomFields(),
      SettingsService.getSecret("ai_api_key").catch(() => ""),
    ]);
    // Inject API key from secure storage into settings for form rendering
    settings.ai_api_key = apiKey;

    this.originalTheme = appState.get("theme");
    this.originalFontSize = settings.font_size || "medium";
    this.originalLibraryRoot = settings.library_root || "";
    const bgImageVal = getComputedStyle(document.documentElement).getPropertyValue("--bg-image").trim();
    this.originalBgImage = bgImageVal && bgImageVal !== "" ? bgImageVal : "none";
    this.originalBgOpacity = settings.bg_opacity || "0.15";
    this.originalBgBlur = settings.bg_blur || "0";
    this.originalBgImagePath = settings.bg_image_path || "";
    this.bgPathModified = false;
    this.pendingBgRemove = false;

    this.overlay = document.createElement("div");
    this.overlay.className = "dialog-overlay";
    this.overlay.addEventListener("click", (e) => {
      if (e.target === this.overlay) this.close();
    });

    const dialog = document.createElement("div");
    dialog.className = "dialog dialog-settings";
    dialog.setAttribute("role", "dialog");
    dialog.setAttribute("aria-modal", "true");
    dialog.setAttribute("aria-label", "Einstellungen");

    // Header
    const header = document.createElement("div");
    header.className = "dialog-header";
    header.innerHTML =
      '<span class="dialog-title">Einstellungen</span>';
    const closeBtn = document.createElement("button");
    closeBtn.className = "dialog-close";
    closeBtn.textContent = "\u00D7";
    closeBtn.setAttribute("aria-label", "Schliessen");
    closeBtn.addEventListener("click", () => this.close());
    header.appendChild(closeBtn);
    dialog.appendChild(header);

    // Body
    const body = document.createElement("div");
    body.className = "dialog-body";

    // Tab bar
    const tabBar = document.createElement("div");
    tabBar.className = "dialog-tab-bar";

    const tabDefs = [
      { key: "general", label: "Allgemein" },
      { key: "appearance", label: "Erscheinungsbild" },
      { key: "ki", label: "KI-Einstellungen" },
      { key: "files", label: "Dateiverwaltung" },
      { key: "custom", label: "Benutzerdefiniert" },
    ];

    for (const def of tabDefs) {
      const tab = document.createElement("button");
      tab.className = "dialog-tab" + (def.key === "general" ? " active" : "");
      tab.textContent = def.label;
      tab.dataset.tab = def.key;
      tabBar.appendChild(tab);
    }
    body.appendChild(tabBar);

    // Tab contents
    const generalForm = this.createTabContent("general", true);
    this.buildGeneralTab(generalForm, settings);
    body.appendChild(generalForm);

    const appearanceForm = this.createTabContent("appearance");
    this.buildAppearanceTab(appearanceForm, settings);
    body.appendChild(appearanceForm);

    const kiForm = this.createTabContent("ki");
    this.buildKiTab(kiForm, settings);
    body.appendChild(kiForm);

    const filesForm = this.createTabContent("files");
    this.buildFilesTab(filesForm, settings);
    body.appendChild(filesForm);

    const customForm = this.createTabContent("custom");
    this.buildCustomTab(customForm, customFields);
    body.appendChild(customForm);

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
      let allOk = true;
      allOk = (await this.saveSettings(generalForm)) && allOk;
      allOk = (await this.saveSettings(appearanceForm)) && allOk;
      allOk = (await this.saveSettings(kiForm)) && allOk;
      allOk = (await this.saveSettings(filesForm)) && allOk;
      // Custom fields are saved inline, not via the save button

      // Execute deferred background image removal on save
      if (allOk && this.pendingBgRemove) {
        try {
          await SettingsService.removeBackgroundImage();
          this.bgPathModified = false;
        } catch (e) {
          console.warn("Failed to remove background:", e);
          allOk = false;
        }
      }

      if (allOk) {
        // Mark bg as committed so cancel won't revert
        this.bgPathModified = false;
        // Update watcher if library_root changed
        const libraryInput = generalForm.querySelector<HTMLInputElement>('[data-key="library_root"]');
        if (libraryInput && libraryInput.value !== this.originalLibraryRoot) {
          try {
            await invoke("watcher_stop");
            if (libraryInput.value) {
              await invoke("watcher_start", { path: libraryInput.value });
            }
          } catch (e) {
            console.warn("Failed to update watcher:", e);
          }
        }
        ToastContainer.show("success", "Einstellungen gespeichert");
        this.close(true);
      } else {
        ToastContainer.show("error", "Einige Einstellungen konnten nicht gespeichert werden");
        saveBtn.disabled = false;
        saveBtn.textContent = "Speichern";
      }
    });
    footer.appendChild(saveBtn);

    dialog.appendChild(footer);
    this.overlay.appendChild(dialog);
    document.body.appendChild(this.overlay);
    this.releaseFocusTrap = trapFocus(dialog);

    // Responsive layout: toggle narrow mode based on dialog width
    this.resizeObserver = new ResizeObserver((entries) => {
      for (const entry of entries) {
        const width = entry.contentRect.width;
        dialog.classList.toggle("dialog-settings--narrow", width < 440);
      }
    });
    this.resizeObserver.observe(dialog);
  }

  private createTabContent(key: string, visible = false): HTMLElement {
    const el = document.createElement("div");
    el.className = "settings-form settings-tab-content";
    el.dataset.tabContent = key;
    if (!visible) el.style.display = "none";
    return el;
  }

  private buildGeneralTab(
    form: HTMLElement,
    settings: Record<string, string>
  ): void {
    // Bibliotheks-Stammverzeichnis
    const libraryGroup = this.createFormGroup("Bibliotheks-Stammverzeichnis");
    const libraryInput = document.createElement("input");
    libraryInput.type = "text";
    libraryInput.className = "settings-input";
    libraryInput.dataset.key = "library_root";
    libraryInput.value = settings.library_root || "";
    libraryInput.placeholder = "~/Stickdateien";
    libraryGroup.appendChild(libraryInput);
    form.appendChild(libraryGroup);

    // Metadaten-Verzeichnis
    const metaGroup = this.createFormGroup("Metadaten-Verzeichnis");
    const metaInput = document.createElement("input");
    metaInput.type = "text";
    metaInput.className = "settings-input";
    metaInput.dataset.key = "metadata_root";
    metaInput.value = settings.metadata_root || "";
    metaInput.placeholder = "~/Stickdateien/.stichman";
    metaGroup.appendChild(metaInput);
    form.appendChild(metaGroup);

    // Migration section
    const migrationGroup = document.createElement("div");
    migrationGroup.className = "settings-form-group settings-migration-group";

    const migrationLabel = document.createElement("label");
    migrationLabel.className = "settings-label";
    migrationLabel.textContent = "Datenimport";
    migrationGroup.appendChild(migrationLabel);

    const migrationDesc = document.createElement("div");
    migrationDesc.className = "settings-legend";
    migrationDesc.textContent =
      "Dateien, Ordner, Metadaten und Tags aus 2stitch Organizer importieren.";
    migrationGroup.appendChild(migrationDesc);

    const migrationBtn = document.createElement("button");
    migrationBtn.className = "dialog-btn dialog-btn-secondary";
    migrationBtn.textContent = "2stitch Import starten";
    migrationBtn.addEventListener("click", () => {
      this.close();
      EventBus.emit("migration:2stitch");
    });
    migrationGroup.appendChild(migrationBtn);

    form.appendChild(migrationGroup);
  }

  private buildAppearanceTab(
    form: HTMLElement,
    settings: Record<string, string>
  ): void {
    // Theme toggle
    const themeGroup = this.createFormGroup("Theme");
    const themeSelect = document.createElement("select");
    themeSelect.className = "settings-input";
    themeSelect.dataset.key = "theme_mode";
    for (const opt of [
      { value: "hell", label: "Hell" },
      { value: "dunkel", label: "Dunkel" },
    ]) {
      const option = document.createElement("option");
      option.value = opt.value;
      option.textContent = opt.label;
      const current = settings.theme_mode || appState.get("theme");
      if (current === opt.value) option.selected = true;
      themeSelect.appendChild(option);
    }
    themeSelect.addEventListener("change", () => {
      const theme = themeSelect.value as ThemeMode;
      document.documentElement.setAttribute("data-theme", theme);
      appState.set("theme", theme);
    });
    themeGroup.appendChild(themeSelect);
    form.appendChild(themeGroup);

    // Font size
    const fontGroup = this.createFormGroup("Schriftgroesse");
    const fontSelect = document.createElement("select");
    fontSelect.className = "settings-input";
    fontSelect.dataset.key = "font_size";
    for (const opt of [
      { value: "small", label: "Klein (12px)" },
      { value: "medium", label: "Mittel (13px)" },
      { value: "large", label: "Gross (15px)" },
    ]) {
      const option = document.createElement("option");
      option.value = opt.value;
      option.textContent = opt.label;
      if ((settings.font_size || "medium") === opt.value) option.selected = true;
      fontSelect.appendChild(option);
    }
    fontSelect.addEventListener("change", () => {
      applyFontSize(fontSelect.value);
    });
    fontGroup.appendChild(fontSelect);
    form.appendChild(fontGroup);

    // Apply current font size
    applyFontSize(settings.font_size || "medium");

    // Background image section
    const bgGroup = document.createElement("div");
    bgGroup.className = "settings-form-group settings-bg-group";

    const bgLabel = document.createElement("label");
    bgLabel.className = "settings-label";
    bgLabel.textContent = "Hintergrundbild";
    bgGroup.appendChild(bgLabel);

    const bgDesc = document.createElement("div");
    bgDesc.className = "settings-legend";
    bgDesc.textContent = "Ein Bild als Hintergrund der Anwendung anzeigen.";
    bgGroup.appendChild(bgDesc);

    const bgPreview = document.createElement("div");
    bgPreview.className = "settings-bg-preview";
    const currentBgImage = getComputedStyle(document.documentElement)
      .getPropertyValue("--bg-image")
      .trim();
    if (currentBgImage && currentBgImage !== "none") {
      bgPreview.style.backgroundImage = currentBgImage;
    } else {
      bgPreview.textContent = "Kein Bild";
    }
    bgGroup.appendChild(bgPreview);

    const bgBtnRow = document.createElement("div");
    bgBtnRow.className = "settings-bg-buttons";

    const bgSelectBtn = document.createElement("button");
    bgSelectBtn.className = "dialog-btn dialog-btn-secondary";
    bgSelectBtn.textContent = "Bild waehlen";
    bgSelectBtn.addEventListener("click", async () => {
      const selected = await open({
        multiple: false,
        title: "Hintergrundbild waehlen",
        filters: [
          {
            name: "Bilder",
            extensions: ["png", "jpg", "jpeg", "webp", "bmp"],
          },
        ],
      });
      if (!selected) return;
      const path = typeof selected === "string" ? selected : String(selected);
      if (!path) return;

      try {
        await SettingsService.copyBackgroundImage(path);
        this.bgPathModified = true;
        this.pendingBgRemove = false;
        const dataUri = await SettingsService.getBackgroundImage();
        if (dataUri) {
          document.documentElement.style.setProperty(
            "--bg-image",
            `url("${dataUri}")`
          );
          bgPreview.style.backgroundImage = `url("${dataUri}")`;
          bgPreview.textContent = "";
          bgRemoveBtn.style.display = "";
        }
      } catch (e) {
        console.warn("Failed to set background:", e);
        ToastContainer.show("error", "Hintergrundbild konnte nicht gesetzt werden");
      }
    });
    bgBtnRow.appendChild(bgSelectBtn);

    const bgRemoveBtn = document.createElement("button");
    bgRemoveBtn.className = "dialog-btn dialog-btn-secondary";
    bgRemoveBtn.textContent = "Bild entfernen";
    bgRemoveBtn.style.display =
      settings.bg_image_path ? "" : "none";
    bgRemoveBtn.addEventListener("click", () => {
      // Defer actual deletion to save — cancel can revert
      this.pendingBgRemove = true;
      document.documentElement.style.setProperty("--bg-image", "none");
      bgPreview.style.backgroundImage = "";
      bgPreview.textContent = "Kein Bild";
      bgRemoveBtn.style.display = "none";
    });
    bgBtnRow.appendChild(bgRemoveBtn);
    bgGroup.appendChild(bgBtnRow);

    // Opacity slider
    const opacityGroup = document.createElement("div");
    opacityGroup.className = "settings-form-group";
    const opacityLabel = document.createElement("label");
    opacityLabel.className = "settings-label";
    opacityLabel.textContent = "Deckkraft";
    opacityGroup.appendChild(opacityLabel);

    const opacityWrapper = document.createElement("div");
    opacityWrapper.className = "settings-range-wrapper";
    const opacitySlider = document.createElement("input");
    opacitySlider.type = "range";
    opacitySlider.className = "settings-range";
    opacitySlider.min = "0.05";
    opacitySlider.max = "0.5";
    opacitySlider.step = "0.05";
    opacitySlider.dataset.key = "bg_opacity";
    opacitySlider.value = settings.bg_opacity || "0.15";
    const opacityDisplay = document.createElement("span");
    opacityDisplay.className = "settings-range-value";
    opacityDisplay.textContent = opacitySlider.value;
    opacitySlider.addEventListener("input", () => {
      opacityDisplay.textContent = opacitySlider.value;
      document.documentElement.style.setProperty(
        "--bg-opacity",
        opacitySlider.value
      );
    });
    opacityWrapper.appendChild(opacitySlider);
    opacityWrapper.appendChild(opacityDisplay);
    opacityGroup.appendChild(opacityWrapper);
    bgGroup.appendChild(opacityGroup);

    // Blur slider
    const blurGroup = document.createElement("div");
    blurGroup.className = "settings-form-group";
    const blurLabel = document.createElement("label");
    blurLabel.className = "settings-label";
    blurLabel.textContent = "Unschaerfe (px)";
    blurGroup.appendChild(blurLabel);

    const blurWrapper = document.createElement("div");
    blurWrapper.className = "settings-range-wrapper";
    const blurSlider = document.createElement("input");
    blurSlider.type = "range";
    blurSlider.className = "settings-range";
    blurSlider.min = "0";
    blurSlider.max = "20";
    blurSlider.step = "1";
    blurSlider.dataset.key = "bg_blur";
    blurSlider.value = settings.bg_blur || "0";
    const blurDisplay = document.createElement("span");
    blurDisplay.className = "settings-range-value";
    blurDisplay.textContent = blurSlider.value;
    blurSlider.addEventListener("input", () => {
      blurDisplay.textContent = blurSlider.value;
      document.documentElement.style.setProperty(
        "--bg-blur",
        blurSlider.value + "px"
      );
    });
    blurWrapper.appendChild(blurSlider);
    blurWrapper.appendChild(blurDisplay);
    blurGroup.appendChild(blurWrapper);
    bgGroup.appendChild(blurGroup);

    form.appendChild(bgGroup);
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
    urlInput.value = settings.ai_url || "";
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
    modelInput.value = settings.ai_model || "";
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
    renameInput.value = settings.rename_pattern || "";
    renameInput.placeholder = "{name}_{theme}";
    renameGroup.appendChild(renameInput);
    form.appendChild(renameGroup);

    // Organisationsmuster
    const organizeGroup = this.createFormGroup("Organisationsmuster");
    const organizeInput = document.createElement("input");
    organizeInput.type = "text";
    organizeInput.className = "settings-input";
    organizeInput.dataset.key = "organize_pattern";
    organizeInput.value = settings.organize_pattern || "";
    organizeInput.placeholder = "{theme}/{name}";
    organizeGroup.appendChild(organizeInput);
    form.appendChild(organizeGroup);
  }

  private buildCustomTab(
    form: HTMLElement,
    customFields: CustomFieldDef[]
  ): void {
    // Existing fields list
    const listContainer = document.createElement("div");
    listContainer.className = "custom-fields-list";
    this.renderCustomFieldsList(listContainer, customFields);
    form.appendChild(listContainer);

    // Create new field form
    const createSection = document.createElement("div");
    createSection.className = "custom-fields-create";

    const sectionHeader = document.createElement("div");
    sectionHeader.className = "settings-legend";
    sectionHeader.innerHTML = "<strong>Neues Feld erstellen</strong>";
    createSection.appendChild(sectionHeader);

    const nameGroup = this.createFormGroup("Feldname");
    const nameInput = document.createElement("input");
    nameInput.type = "text";
    nameInput.className = "settings-input";
    nameInput.placeholder = "z.B. Schwierigkeitsgrad";
    nameGroup.appendChild(nameInput);
    createSection.appendChild(nameGroup);

    const typeGroup = this.createFormGroup("Typ");
    const typeSelect = document.createElement("select");
    typeSelect.className = "settings-input";
    for (const opt of [
      { value: "text", label: "Text" },
      { value: "number", label: "Zahl" },
      { value: "date", label: "Datum" },
      { value: "select", label: "Auswahl" },
    ]) {
      const option = document.createElement("option");
      option.value = opt.value;
      option.textContent = opt.label;
      typeSelect.appendChild(option);
    }
    typeGroup.appendChild(typeSelect);
    createSection.appendChild(typeGroup);

    const optionsGroup = this.createFormGroup("Optionen (kommagetrennt)");
    optionsGroup.className = "settings-form-group custom-field-options-group";
    optionsGroup.style.display = "none";
    const optionsInput = document.createElement("input");
    optionsInput.type = "text";
    optionsInput.className = "settings-input";
    optionsInput.placeholder = "Leicht, Mittel, Schwer";
    optionsGroup.appendChild(optionsInput);
    createSection.appendChild(optionsGroup);

    typeSelect.addEventListener("change", () => {
      optionsGroup.style.display =
        typeSelect.value === "select" ? "" : "none";
    });

    const createBtn = document.createElement("button");
    createBtn.className = "dialog-btn dialog-btn-primary";
    createBtn.textContent = "Feld erstellen";
    createBtn.style.marginTop = "8px";
    createBtn.addEventListener("click", async () => {
      const name = nameInput.value.trim();
      if (!name) {
        ToastContainer.show("error", "Feldname darf nicht leer sein");
        return;
      }
      createBtn.disabled = true;
      try {
        const options =
          typeSelect.value === "select" ? optionsInput.value.trim() || null : null;
        const field = await SettingsService.createCustomField(
          name,
          typeSelect.value,
          options ?? undefined
        );
        customFields.push(field);
        this.renderCustomFieldsList(listContainer, customFields);
        nameInput.value = "";
        optionsInput.value = "";
        ToastContainer.show("success", `Feld "${name}" erstellt`);
      } catch (e) {
        console.warn("Failed to create custom field:", e);
        ToastContainer.show("error", "Feld konnte nicht erstellt werden");
      }
      createBtn.disabled = false;
    });
    createSection.appendChild(createBtn);
    form.appendChild(createSection);
  }

  private renderCustomFieldsList(
    container: HTMLElement,
    fields: CustomFieldDef[]
  ): void {
    container.innerHTML = "";

    if (fields.length === 0) {
      const empty = document.createElement("div");
      empty.className = "custom-fields-empty";
      empty.textContent = "Keine benutzerdefinierten Felder vorhanden";
      container.appendChild(empty);
      return;
    }

    for (const field of fields) {
      const row = document.createElement("div");
      row.className = "custom-field-row";

      const info = document.createElement("div");
      info.className = "custom-field-info";

      const nameEl = document.createElement("span");
      nameEl.className = "custom-field-name";
      nameEl.textContent = field.name;
      info.appendChild(nameEl);

      const typeEl = document.createElement("span");
      typeEl.className = "custom-field-type";
      typeEl.textContent = field.fieldType;
      if (field.options) {
        typeEl.textContent += ` (${field.options})`;
      }
      info.appendChild(typeEl);

      row.appendChild(info);

      const deleteBtn = document.createElement("button");
      deleteBtn.className = "dialog-btn dialog-btn-danger";
      deleteBtn.textContent = "Loeschen";
      deleteBtn.style.padding = "2px 8px";
      deleteBtn.style.fontSize = "var(--font-size-caption)";
      deleteBtn.addEventListener("click", async () => {
        if (!confirm(`Feld "${field.name}" wirklich loeschen?`)) return;
        try {
          await SettingsService.deleteCustomField(field.id);
          const idx = fields.indexOf(field);
          if (idx >= 0) fields.splice(idx, 1);
          this.renderCustomFieldsList(container, fields);
          ToastContainer.show("success", `Feld "${field.name}" geloescht`);
        } catch (e) {
          console.warn("Failed to delete custom field:", e);
          ToastContainer.show("error", "Feld konnte nicht geloescht werden");
        }
      });
      row.appendChild(deleteBtn);

      container.appendChild(row);
    }
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

      // API key uses OS keychain via set_secret, not plain settings
      if (key === "ai_api_key") {
        const provider = form.querySelector<HTMLSelectElement>(
          '[data-key="ai_provider"]'
        );
        const value = provider && provider.value !== "openai" ? "" : input.value;
        try {
          await SettingsService.setSecret(key, value);
        } catch (e) {
          console.warn(`Failed to save secret '${key}':`, e);
          allOk = false;
        }
        continue;
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

  private close(saved = false): void {
    if (!saved) {
      // Revert live-preview changes
      document.documentElement.setAttribute("data-theme", this.originalTheme);
      appState.set("theme", this.originalTheme);
      applyFontSize(this.originalFontSize);
      // Revert background changes
      document.documentElement.style.setProperty("--bg-image", this.originalBgImage);
      document.documentElement.style.setProperty("--bg-opacity", this.originalBgOpacity);
      document.documentElement.style.setProperty("--bg-blur", this.originalBgBlur + "px");
      // Undo any background image changes from copyBackgroundImage
      if (this.bgPathModified) {
        // Remove the orphaned copied file, then restore the original DB path
        SettingsService.removeBackgroundImage().catch(() => {});
        if (this.originalBgImagePath) {
          SettingsService.setSetting("bg_image_path", this.originalBgImagePath).catch(() => {});
        }
      }
    }
    if (this.resizeObserver) {
      this.resizeObserver.disconnect();
      this.resizeObserver = null;
    }
    if (this.releaseFocusTrap) {
      this.releaseFocusTrap();
      this.releaseFocusTrap = null;
    }
    if (this.overlay) {
      this.overlay.remove();
      this.overlay = null;
    }
    SettingsDialog.instance = null;
  }
}
