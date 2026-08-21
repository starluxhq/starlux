# React Architecture & Rules

## Core Stack
- React 19 / Vite / TypeScript / Tailwind CSS
- State: TanStack Query (server state), Zustand (client state)

## Code Standards
- Prefer composition and small, pure UI components over large monoliths.
- Avoid raw `useEffect` for data fetching; use TanStack Query hooks.
- Keep components typed with explicit interface props.

## Verification Workflow
- Type check: `npx tsc --noEmit`
- React Audit: `npx react-doctor@latest --scope changed`
- Test: `npm test`
