# TC-19: File Watcher

## TC-19-01: Watcher Auto-Start
- **Steps:** Set library_root in settings, restart app
- **Expected:** File watcher starts monitoring the directory
- **Status:** PENDING

## TC-19-02: Auto-Import New File
- **Steps:** Copy a .PES file into watched directory
- **Expected:** Toast "1 neue Datei(en) importiert", file appears in list
- **Status:** PENDING

## TC-19-03: Auto-Remove Deleted File
- **Steps:** Delete a file from watched directory
- **Expected:** Toast "1 Datei(en) entfernt", file removed from list
- **Status:** PENDING

## TC-19-04: Watcher Debounce
- **Steps:** Copy multiple files rapidly
- **Expected:** Events batched (500ms debounce), no duplicate imports
- **Status:** PENDING

## TC-19-05: Watcher Stop/Start
- **Steps:** Change library_root in settings
- **Expected:** Old watcher stops, new one starts on new path
- **Status:** PENDING
