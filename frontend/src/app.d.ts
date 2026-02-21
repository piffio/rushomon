// See https://svelte.dev/docs/kit/types#app.d.ts
// for information about these interfaces
declare global {
	namespace App {
		// interface Error {}
		// interface Locals {}
		interface PageData {
			// Dashboard page data
			initialSearch?: string;
			initialStatus?: 'all' | 'active' | 'disabled';
			initialSort?: 'created' | 'updated' | 'clicks' | 'title' | 'code';
		}
		// interface PageState {}
		// interface Platform {}
	}
}

export { };
