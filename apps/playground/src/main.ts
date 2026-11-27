import { mount } from 'svelte';
// Only the weights and the Latin subset the page uses; other characters fall back per glyph.
import '@fontsource/ibm-plex-sans/latin-400.css';
import '@fontsource/ibm-plex-sans/latin-500.css';
import '@fontsource/ibm-plex-sans/latin-600.css';
import '@fontsource/victor-mono/latin-400.css';
import '@fontsource/victor-mono/latin-500.css';
import '@fontsource/victor-mono/latin-500-italic.css';
import './tokens.css';
import './app.css';
import App from './App.svelte';

const app = mount(App, { target: document.getElementById('app')! });

export default app;
