# Rushomon Frontend

SvelteKit frontend for the Rushomon URL shortener.

## Development

```bash
# Install dependencies
npm install

# Start dev server (requires backend on port 8787)
npm run dev

# Build for production
npm run build

# Preview production build
npm run preview
```

## Environment Variables

Create a `.env` file:

```
VITE_API_BASE_URL=http://localhost:8787
```

For production, set this to your deployed Worker URL.

## Backend Requirements

The backend Worker must be running for the frontend to function:

```bash
# In project root
wrangler dev --local --port 8787
```

The frontend expects the backend API at the URL specified in `VITE_API_BASE_URL`.

## Building for Cloudflare Pages

```bash
npm run build
```

The build output will be in `.svelte-kit/cloudflare/`.

Deploy with:

```bash
wrangler pages deploy .svelte-kit/cloudflare
```

## Project Structure

```
src/
├── lib/
│   ├── api/          # API client functions
│   ├── components/   # Svelte components
│   ├── stores/       # Svelte stores
│   └── types/        # TypeScript types
├── routes/
│   ├── +layout.svelte     # Root layout
│   ├── +page.svelte       # Landing page
│   └── dashboard/
│       ├── +page.ts       # Load function with auth check
│       └── +page.svelte   # Dashboard page
└── app.css           # Global styles (Tailwind)
```

## Authentication Flow

1. User clicks "Sign in with GitHub" on landing page
2. Redirects to `/api/auth/github` (backend)
3. Backend redirects to GitHub OAuth
4. GitHub redirects back to `/api/auth/callback` (backend)
5. Backend sets httpOnly session cookie and redirects to `/dashboard`
6. Dashboard `+page.ts` loader checks authentication via `/api/auth/me`
7. If authenticated, loads user data and links
8. If not authenticated, redirects back to landing page

## Components

### Header.svelte
Navigation header with user menu and logout.

### CreateLinkForm.svelte
Form for creating new short links with validation.

### LinkList.svelte
Displays paginated list of links with loading/empty states.

### LinkCard.svelte
Individual link card with copy, click count, and delete actions.

## API Integration

All API calls go through `src/lib/api/`:
- `client.ts` - Base API client with error handling
- `auth.ts` - Authentication endpoints
- `links.ts` - Link CRUD endpoints

Requests include `credentials: 'include'` to send httpOnly cookies.

## Styling

Uses Tailwind CSS with a minimal, clean design:
- Gray-based color scheme
- Responsive (mobile-first)
- Hover states and transitions
- Shadow and rounded corners for depth

## Testing

Manual testing checklist in `/docs/TESTING_GUIDE.md`.

For automated testing (future):
- E2E tests with Playwright
- Component tests with Vitest
- Accessibility tests with axe