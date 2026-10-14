import { mount } from 'svelte';
import App from './App.svelte';
import './app.css';
import './components.css';

const target = document.getElementById('app');
if (!target) {
  throw new Error('Missing #app element');
}

export default mount(App, { target });
