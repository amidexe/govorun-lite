import { mount } from 'svelte';
import RecordingIndicator from '../features/dictation/RecordingIndicator.svelte';
import { recordingIndicatorApi } from '../desktop-api';
mount(RecordingIndicator, {
  target: document.getElementById('app')!,
  props: { api: recordingIndicatorApi() },
});
