# Anforderungsdokument
## Manufaktur-Projekt Nähen und Sticken

**Dokumentname:** Projektanforderungen Nähen und Sticken  
**Dateiname:** project.md  
**Version:** 1.0  
**Status:** Entwurf  
**Datum:** 2026-03-16

---

## 1. Zielsetzung

Ziel dieses Projekts ist die strukturierte Planung, Steuerung, Dokumentation und Kalkulation von Manufakturprojekten im Bereich **Nähen und Sticken**. Das System soll sämtliche relevanten Informationen entlang des gesamten Projekt- und Produktionsprozesses erfassen, auswerten und nachvollziehbar machen.

Dabei müssen insbesondere folgende Bereiche berücksichtigt werden:

- Materialverwaltung
- Zeit- und Arbeitsaufwand
- Lizenzverwaltung
- Bestellungen und Beschaffung
- Arbeits- und Prozessschritte
- Prozessbeschreibung und Nachverfolgbarkeit
- Netto-Preis- und Verkaufspreiskalkulation

Das Anforderungsdokument dient als Grundlage für die fachliche Konzeption, Systementwicklung oder Auswahl einer geeigneten Softwarelösung.

---

## 2. Geltungsbereich

Der Geltungsbereich umfasst insbesondere:

- Einzelanfertigungen
- Kleinserien
- personalisierte Produkte
- Nähprojekte
- Stickprojekte
- kombinierte Näh- und Stickprojekte
- interne Entwicklungsprojekte
- Kundenaufträge
- lager- und beschaffungsbezogene Prozesse

Optional erweiterbar:

- Versandabwicklung
- Kundenverwaltung
- Rechnungsstellung
- Anbindung an Buchhaltungssysteme

---

## 3. Fachliche Zielobjekte

Das System muss folgende Kernobjekte verwalten können:

### 3.1 Projekte / Aufträge

Für jedes Projekt bzw. jeden Auftrag müssen mindestens folgende Daten gepflegt werden können:

- Projekt- oder Auftragsnummer
- Projektname
- Kunde / Auftraggeber
- Produktbezug
- Projektstatus
- Startdatum
- Zieltermin / Liefertermin
- Priorität
- verantwortliche Person
- verknüpfte Materialien
- verknüpfte Dateien
- verknüpfte Lizenzen
- geplanter und tatsächlicher Aufwand
- Kalkulationsstatus
- Freigabestatus

### 3.2 Produkte

Für Produkte müssen mindestens folgende Informationen erfasst werden können:

- Produktnummer
- Produktname
- Kategorie
- Beschreibung
- Produktart:
  - Nähprodukt
  - Stickprodukt
  - Kombinationsprodukt
- Varianten
- Größen
- Farben
- Status

### 3.3 Materialien

Für Materialien müssen Stammdaten und Lagerinformationen verwaltet werden:

- Materialnummer
- Materialbezeichnung
- Materialart
- Einheit
- Lieferant
- Netto-Einkaufspreis
- aktueller Lagerbestand
- reservierter Bestand
- verfügbarer Bestand
- Mindestbestand
- Lagerort
- Wiederbeschaffungszeit
- Schwund- bzw. Verschnittfaktor

### 3.4 Dateien und Vorlagen

Folgende Dateien müssen verknüpfbar und verwaltbar sein:

- Schnittmuster
- Stickdateien
- Motivdateien
- Arbeitsanweisungen
- Produktfotos
- Pflegehinweise
- Kundenvorlagen

### 3.5 Lizenzen

Das System muss Lizenzen für Designs, Motive, Schnittmuster und Vorlagen verwalten können.

Erfassbare Informationen:

- Lizenzgeber
- Lizenzart
- Gültigkeitszeitraum
- kommerzielle Nutzung erlaubt / nicht erlaubt
- Stückzahlbegrenzung
- Lizenzdokument
- zugeordnete Datei / Design
- Status

---

## 4. Rollen und Berechtigungen

Das System soll mindestens folgende Rollen unterstützen:

- Projektleitung
- Einkauf / Beschaffung
- Produktion Nähen
- Produktion Sticken
- Lagerverwaltung
- Kalkulation / Vertrieb
- Qualitätskontrolle
- Administration

Je Rolle müssen mindestens folgende Berechtigungsarten definierbar sein:

- lesen
- anlegen
- bearbeiten
- freigeben
- bestellen
- kalkulieren
- archivieren

---

## 5. Funktionale Anforderungen

## 5.1 Materialverwaltung

Das System muss:

1. Materialien strukturiert erfassen und pflegen können.
2. Materialverbräuche pro Projekt und pro Stück erfassen können.
3. Verschnitt, Schwund und Ausschuss berücksichtigen können.
4. Lagerbestände automatisch reservieren, reduzieren und freigeben können.
5. Mindestbestände überwachen können.
6. Materialbedarf projektbezogen ermitteln können.
7. Alternativmaterialien verwalten können.
8. Materialkosten pro Produkt und Projekt berechnen können.

### Muss-Anforderungen

- Stücklisten pro Produkt
- Materialverbrauch je Arbeitsschritt
- automatische Reservierung bei Projektfreigabe
- Warnung bei Unterschreitung von Mindestbeständen
- Nachkalkulation mit Ist-Verbrauch

---

## 5.2 Zeit- und Arbeitsaufwand

Das System muss geplante und tatsächliche Zeiten erfassen können.

Zu berücksichtigen sind insbesondere:

- Zuschnittzeit
- Stickvorbereitung
- Maschinenlaufzeit Stickmaschine
- Nähzeit
- Nachbearbeitung
- Qualitätskontrolle
- Verpackung
- Rüstzeiten

### Anforderungen

- Soll-Zeiten pro Arbeitsschritt
- Ist-Zeiten pro Arbeitsschritt
- Zeitbuchung pro Mitarbeiter oder Arbeitsplatz
- Zeitkostensatz pro Ressource
- Maschinenstundensatz
- Auswertung von Abweichungen zwischen Soll und Ist

---

## 5.3 Lizenzverwaltung

Das System muss Nutzungsrechte und Lizenzbeschränkungen abbilden können.

### Anforderungen

1. Lizenzgeber müssen erfasst werden können.
2. Lizenzdokumente müssen hinterlegt werden können.
3. Gültigkeitszeiträume müssen überwacht werden können.
4. Nutzungsbeschränkungen müssen gespeichert werden können.
5. maximale Stückzahlen je Lizenz müssen verwaltet werden können.
6. Warnungen bei unzulässiger Nutzung müssen ausgegeben werden können.
7. je Produkt muss nachvollziehbar sein, welche lizenzierten Dateien verwendet wurden.

### Muss-Anforderungen

- Verknüpfung von Lizenz und Datei
- Warnung bei abgelaufener Lizenz
- Kennzeichnung „für Verkauf zulässig“ / „nicht für Verkauf zulässig“

---

## 5.4 Bestellungen und Beschaffung

Das System muss den Beschaffungsprozess unterstützen.

### Anforderungen

- projektbezogene Bedarfsermittlung
- Bestellvorschläge
- Lieferantenverwaltung
- Preisverwaltung je Lieferant
- Pflege von Lieferzeiten
- Anlegen und Verfolgen von Bestellungen
- Verwaltung von Teil- und Restlieferungen
- Buchung von Wareneingängen
- Zuordnung von Beschaffungen zu Projekten oder Lager
- Einbeziehung von Beschaffungskosten in die Kalkulation

### Muss-Anforderungen

- Bestellstatus mit mindestens:
  - angefragt
  - bestellt
  - teilweise geliefert
  - vollständig geliefert
  - storniert
- Verknüpfung zwischen Bestellung und Projekt
- Überwachung geplanter Liefertermine

---

## 5.5 Arbeitsgänge und Produktionsschritte

Das System muss Herstellungsprozesse in definierte Arbeitsschritte gliedern können.

### Beispielhafte Schritte

1. Auftrag anlegen
2. Anforderungen prüfen
3. Materialbedarf ermitteln
4. Material reservieren oder beschaffen
5. Dateien und Lizenzen prüfen
6. Zuschnitt
7. Stickvorbereitung
8. Stickprozess
9. Nähprozess
10. Nachbearbeitung
11. Qualitätskontrolle
12. Verpackung
13. Einlagerung oder Versand
14. Abschluss und Nachkalkulation

### Anforderungen

- frei definierbare Prozessschritte
- Reihenfolge und Abhängigkeiten
- Pflichtschritte
- Verantwortlichkeit pro Schritt
- Bearbeitungsstatus pro Schritt
- Dokumentation von Abweichungen
- Zeit- und Materialverbrauch pro Schritt
- Freigabepunkte im Prozess

---

## 5.6 Projektplanung und Steuerung

Das System muss Projekte planbar und steuerbar machen.

### Anforderungen

- Terminplanung
- Meilensteine
- Kapazitätsplanung
- Priorisierung
- Ressourcenzuordnung
- Statusübersicht
- Soll-/Ist-Vergleiche
- Warnungen bei Terminverzug oder Materialengpässen

---

## 5.7 Qualitätsmanagement

Das System soll Qualitätsprüfungen innerhalb des Herstellungsprozesses unterstützen.

### Anforderungen

- Definition von Prüfmerkmalen
- Prüfschritte je Projekt / Produkt
- Dokumentation von Fehlern
- Erfassung von Nacharbeit
- Erfassung von Ausschuss
- Freigabeentscheidung
- optionale Fotodokumentation

---

## 6. Prozessbeschreibung

## 6.1 Gesamtprozess

### Phase 1 – Anfrage / Produktidee
- Kundenanforderung oder Produktidee erfassen
- Produkt, Variante und Individualisierung definieren
- Dateien, Designs und Lizenzen zuordnen

### Phase 2 – Planung
- Materialbedarf ermitteln
- Arbeitsgänge und Zeitbedarf planen
- Verfügbarkeit von Material und Lizenz prüfen
- Netto-Kalkulation und Verkaufspreis berechnen

### Phase 3 – Beschaffung
- Fehlende Materialien identifizieren
- Bestellungen auslösen
- Liefertermine überwachen
- Wareneingänge erfassen

### Phase 4 – Produktion
- Material reservieren und entnehmen
- Zuschnitt, Stickerei und Nähen durchführen
- Ist-Zeiten und Ist-Verbräuche dokumentieren
- Qualitätsprüfungen durchführen

### Phase 5 – Abschluss
- Produkt fertigstellen
- verpacken, einlagern oder versenden
- Nachkalkulation durchführen
- Projekt archivieren
- Kennzahlen aktualisieren

---

## 7. Anforderungen an die Kalkulation

## 7.1 Ziel der Kalkulation

Das System muss aus den erfassten Projektinformationen eine belastbare **Netto-Kalkulation** sowie eine **Verkaufspreiskalkulation** erzeugen können.

Dabei müssen mindestens folgende Kostenbestandteile berücksichtigt werden:

- Materialkosten
- Lizenzkosten
- direkte Arbeitskosten
- Maschinenkosten
- Beschaffungskosten
- Verpackungskosten
- Gemeinkosten
- Ausschuss- und Schwundzuschläge
- Gewinnzuschlag

---

## 7.2 Kalkulationslogik

### 7.2.1 Materialkosten netto

**Materialkosten netto = Summe aller eingesetzten Materialien × Netto-Einstandspreis + Verschnittzuschlag**

Zu berücksichtigen sind:

- Stoffe
- Garne
- Stickgarne
- Vliese
- Reißverschlüsse
- Knöpfe
- Etiketten
- Verpackungsmaterial
- sonstiges Zubehör

### 7.2.2 Lizenzkosten netto

Lizenzkosten müssen abbildbar sein als:

- Kosten pro Stück
- anteilige Kosten pro Serie
- pauschale Projektkosten

**Lizenzkosten pro Stück = Gesamtlizenzkosten / nutzbare Stückzahl**

### 7.2.3 Arbeitskosten netto

**Arbeitskosten = Summe der Bearbeitungszeiten × Stundensatz**

Zu berücksichtigen sind mindestens:

- Zuschnitt
- Stickvorbereitung
- Näharbeit
- Nachbearbeitung
- Qualitätskontrolle
- Verpackung

### 7.2.4 Maschinenkosten netto

**Maschinenkosten = Maschinenlaufzeit × Maschinenstundensatz + Rüstkostenanteil**

Zu berücksichtigen sind insbesondere:

- Stickmaschine
- Nähmaschine
- ggf. weitere Spezialmaschinen

### 7.2.5 Beschaffungskosten netto

Einzubeziehen sind insbesondere:

- Versandkosten Einkauf
- Importkosten optional
- Expresszuschläge
- Mindestmengenzuschläge
- Sonderbeschaffungskosten

### 7.2.6 Gemeinkosten

Gemeinkosten müssen als Zuschlagssatz oder fixer Kostenblock berücksichtigt werden können.

Beispiele:

- Strom
- Miete
- Verwaltung
- Instandhaltung
- Softwarekosten
- Abschreibung
- Reinigung

**Gemeinkosten = Zuschlagssatz × Herstellkosten**

---

## 7.3 Verkaufspreiskalkulation netto

Das System muss aus den Selbstkosten einen Netto-Verkaufspreis ableiten können.

### Kalkulationsschema

1. Materialkosten
2. + Lizenzkosten
3. + direkte Arbeitskosten
4. + Maschinenkosten
5. + Beschaffungskosten
6. + Verpackungskosten
7. + Gemeinkosten
= **Selbstkosten netto**

8. + Gewinnzuschlag
= **Netto-Verkaufspreis**

Optional:

9. + Rabattpuffer / Händlermarge
10. + Umsatzsteuer
= **Brutto-Verkaufspreis**

---

## 7.4 Beispiel einer Netto-Kalkulation

### Beispielprodukt: Bestickte Kosmetiktasche

**Materialkosten:**
- Oberstoff: 0,5 m × 10,00 € = 5,00 €
- Futterstoff: 0,3 m × 8,00 € = 2,40 €
- Reißverschluss = 1,20 €
- Stickvlies = 0,70 €
- Stickgarn = 0,90 €
- Etikett = 0,30 €
- Verpackung = 0,50 €

Materialsumme: **11,00 €**

Verschnitt 7 %: **0,77 €**

**Materialkosten gesamt:** **11,77 €**

**Lizenzkosten pro Stück:** **1,20 €**

**Arbeitskosten:**
42 min × 36,00 €/h = **25,20 €**

**Maschinenkosten:**
15 min × 12,00 €/h = **3,00 €**

**Beschaffungskostenanteil:** **0,80 €**

**Zwischensumme direkte Kosten:**
11,77 € + 1,20 € + 25,20 € + 3,00 € + 0,80 € = **41,97 €**

**Gemeinkosten 15 %:** **6,30 €**

**Selbstkosten netto:** **48,27 €**

**Gewinnzuschlag 25 %:** **12,07 €**

**Netto-Verkaufspreis:** **60,34 €**

---

## 8. Berichte und Auswertungen

Das System muss mindestens folgende Auswertungen ermöglichen:

- Materialverbrauch je Projekt
- Zeitverbrauch je Projekt
- Soll-/Ist-Kalkulation
- Lizenznutzung
- Bestellstatus
- Lieferantenübersicht
- Marge pro Produkt
- Deckungsbeitrag pro Auftrag
- Ausschussquote
- Nacharbeitsquote
- Lagerreichweite

---

## 9. Nichtfunktionale Anforderungen

### 9.1 Nachvollziehbarkeit
- Änderungen an Kalkulationen, Materialien, Lizenzdaten und Projektstatus müssen nachvollziehbar dokumentiert werden.

### 9.2 Bedienbarkeit
- Die Bedienung muss für kleine und mittlere Manufakturen praktikabel und übersichtlich sein.

### 9.3 Flexibilität
- Materialarten, Arbeitsschritte, Zuschlagssätze und Lizenzarten müssen konfigurierbar sein.

### 9.4 Datenintegrität
- Pflichtfelder, Plausibilitätsprüfungen und Statuslogiken müssen Fehleingaben minimieren.

### 9.5 Performance
- Kalkulationen und Bestandsprüfungen müssen performant ausführbar sein.

### 9.6 Exportfähigkeit
- Kalkulationen, Stücklisten, Projektakten und Bestellübersichten sollen exportierbar sein.

---

## 10. Akzeptanzkriterien

Das Projekt gilt als fachlich erfüllt, wenn mindestens nachgewiesen ist, dass:

1. ein Produkt mit Material, Zeitwerten, Dateien und Lizenzen vollständig angelegt werden kann,
2. Materialbedarf und Lagerverfügbarkeit ermittelt werden können,
3. fehlende Materialien als Beschaffungsbedarf erkannt werden,
4. Bestellungen projektbezogen angelegt und verfolgt werden können,
5. Produktionsschritte dokumentiert werden können,
6. Ist-Zeiten und Ist-Verbräuche erfasst werden können,
7. daraus eine Netto-Selbstkostenkalkulation erstellt werden kann,
8. daraus ein Netto-Verkaufspreis berechnet werden kann,
9. lizenzkritische Nutzungen erkannt werden,
10. der gesamte Projektverlauf nachvollziehbar dokumentiert werden kann.

---

## 11. Empfohlene Systemmodule

Für eine spätere Umsetzung wird folgende Modulstruktur empfohlen:

1. Stammdatenverwaltung
2. Projekt- und Auftragsverwaltung
3. Material- und Lagerverwaltung
4. Datei- und Lizenzverwaltung
5. Bestellungen und Beschaffung
6. Produktionssteuerung
7. Zeit- und Leistungserfassung
8. Kalkulation
9. Qualität und Freigabe
10. Reporting und Auswertung

---

## 12. Zusammenfassung

Das geplante Manufaktur-System für Nähen und Sticken muss in der Lage sein, Projekte ganzheitlich von der Idee bis zur Nachkalkulation abzubilden. Zentrale Anforderungen sind die strukturierte Verwaltung von Materialien, Arbeitszeiten, Lizenzen, Beschaffungsvorgängen und Produktionsschritten sowie eine transparente Netto-Kosten- und Verkaufspreiskalkulation.

Das Dokument bildet die fachliche Grundlage für die weitere Spezifikation, Priorisierung und Umsetzung.
