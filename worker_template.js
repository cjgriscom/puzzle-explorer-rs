import init, { worker_handle_msg } from './puzzle_explorer.js?v=__BUILD_HASH__';

// Start init immediately but don't wait — capture the promise
const ready = init({ module_or_path: './puzzle_explorer_bg.wasm?v=__BUILD_HASH__' });

// Register the listener synchronously so no messages are dropped
self.addEventListener('message', async event => {
	await ready; // Wait for wasm to be loaded before handling
	try {
		const result = worker_handle_msg(event.data);
		self.postMessage({ type: 'success', result });
	} catch (e) {
		console.error("Worker error:", e);
		self.postMessage({ type: 'error', error: e.toString() });
	}
});
