# Backlog — Issue #29: Additional Requirements

**Status:** Requires decomposition into separate issues
**Effort:** XL (epic-level)

---

## Overview

Issue #29 is a comprehensive wishlist covering file conversion, editing, collaboration, machine integration, and more. Most items are large features that need their own issues.

## Triage: What's already covered vs. what's new

### Already covered by existing issues or implemented
| Requirement | Status |
|-------------|--------|
| Upload/Import stitch files | Implemented (scanner, mass import) |
| Preview stitch patterns | Implemented (MetadataPanel canvas preview) |
| Organize: folders, tags, metadata | Implemented |
| Search by name, tags, metadata | Implemented (SearchBar + cross-folder search) |
| Export to machine formats | Implemented (USB export) |
| Color picker/modification | Partially covered by #30 (thread color mapping) |

### Needs new issues (recommended decomposition)

| Requirement | Suggested Issue Title | Priority | Effort |
|-------------|----------------------|----------|--------|
| File format conversion (PES↔DST↔JEF↔VP3) | "File format conversion between supported formats" | Medium | XL |
| Edit: resize, rotate, mirror patterns | "Basic stitch pattern editing (resize, rotate, mirror)" | Low | XL |
| Dashboard/overview page | "Dashboard with recent projects and favorites" | Low | L |
| Interactive editing workspace/canvas | "Interactive stitch editor workspace" | Very Low | XXL |
| Stitch simulation | "Stitch-out simulation/animation" | Low | XL |
| Auto-digitizing (image → stitch) | "Auto-digitize: convert images to stitch patterns" | Very Low | XXL |
| Templates library | "Built-in design templates library" | Low | M |
| Version control for designs | "Design version history" | Low | L |
| Share via link/email/cloud | "Cloud sharing and collaboration" | Very Low | XL |
| Direct machine transfer (Wi-Fi) | "Direct Wi-Fi transfer to embroidery machines" | Low | L |
| Responsive/tablet design | "Responsive layout for tablet screens" | Low | L |
| Mobile app | "Mobile companion app" | Very Low | XXL |
| In-app tutorials | "In-app onboarding tutorials" | Low | M |

### Not applicable to this project
| Requirement | Reason |
|-------------|--------|
| Cloud storage backend | Tauri desktop app, local-first architecture |
| Secure user authentication | Single-user desktop app |
| Role-based access control | Single-user desktop app |
| React/Angular frontend | Vanilla TS by design |

## Recommendation

1. Close #29 as an epic/umbrella issue
2. Create individual issues for the top-priority items from the decomposition
3. Suggested first picks for future sprints:
   - File format conversion (high user value)
   - Basic editing (resize/rotate/mirror — high user value)
   - Dashboard (low effort, nice UX improvement)
