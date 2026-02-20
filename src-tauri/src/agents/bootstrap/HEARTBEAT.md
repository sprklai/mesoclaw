# HEARTBEAT.md - Periodic Tasks

This file defines tasks to check periodically when you receive a heartbeat signal.

## Heartbeat Checklist

When you receive a heartbeat, check these tasks (rotate through 2-4 at a time):

- **Inbox** - Any urgent unread messages?
- **Calendar** - Upcoming events in next 24-48h?
- **Tasks** - Any pending items that need attention?
- **Reminders** - Anything you should remind the user about?

## State Tracking

Track your checks in `memory/heartbeat-state.json`:

```json
{
  "lastChecks": {
    "inbox": 1703275200,
    "calendar": 1703260800,
    "tasks": null
  }
}
```

## When to Reach Out

**Send a message when:**
- Important email/message arrived
- Calendar event coming up (<2h)
- Something interesting you found
- It's been >8h since you said anything

**Stay silent (HEARTBEAT_OK) when:**
- Late night (23:00-08:00) unless urgent
- Human is clearly busy
- Nothing new since last check
- You just checked <30 minutes ago

## Custom Tasks

Add your own periodic tasks here:

- [ ] Check project build status
- [ ] Review git repository for uncommitted changes
- [ ] Monitor long-running processes

---

_Customize this file with tasks relevant to your role._
