<script lang="ts">
  import { useDesktopApi } from '../../shared/state/desktop-context';
  import Bird from '../../shared/ui/Bird.svelte';
  import Icon from '../../shared/ui/Icon.svelte';
  import Modal from '../../shared/ui/Modal.svelte';
  import StatisticsReset from '../statistics/StatisticsReset.svelte';
  import EraseLocalData from './EraseLocalData.svelte';
  import UpdateCard from '../updates/UpdateCard.svelte';
  import { aboutSnapshot } from './model';
  import { desktopBuild } from '../../build';
  import { useNativeSnapshot } from '../../shared/state/native-snapshot.svelte';
  let { platform = 'windows' }: { platform?: 'macos' | 'windows' } = $props();
  const api = useDesktopApi();
  const info = useNativeSnapshot('about.snapshot', aboutSnapshot, 'about.changed');
  let licenses = $state(false),
    error = $state(false),
    busy = $state(false);
  async function open(id: string) {
    if (busy || info.failed) return;
    error = false;
    busy = true;
    try {
      await api.invoke('about.openLink', { id });
    } catch {
      error = true;
    } finally {
      busy = false;
    }
  }
  const uiLicenses = [
    {
      id: 'tauri',
      name: 'Tauri',
      spdx: 'MIT / Apache-2.0',
      copyright: 'Copyright © 2017–present Tauri Apps Contributors',
    },
    {
      id: 'svelte',
      name: 'Svelte',
      spdx: 'MIT',
      copyright: 'Copyright © 2016–2025 Svelte Contributors',
    },
  ];
</script>

<section class="surface identity">
  <Bird size={52} />
  <div>
    <h2>Говорун</h2>
    <p>Версия {info.value?.version ?? desktopBuild.version}</p>
    <p class="secondary">Сборка {info.value?.buildID || desktopBuild.buildID}</p>
    <p class="secondary">Голосовой ввод без интернета</p>
  </div>
</section>
<UpdateCard />
{#if info.failed}<div class="notice" role="alert">
    Не удалось получить сведения о движке. <button onclick={info.refresh}>Повторить</button>
  </div>{/if}
{#if error && !licenses}<p class="error" role="alert">
    Не удалось открыть ссылку. Попробуйте ещё раз.
  </p>{/if}
<section class="surface">
  <h2>Приватность</h2>
  <div class="fact">
    <Icon name="ph_waveform" />
    <div>
      <strong>Распознавание речи</strong>
      <p>Звук обрабатывается локально и не покидает устройство.</p>
    </div>
    <span>Офлайн</span>
  </div>
  <div class="fact">
    <Icon name="ph_sparkle" />
    <div>
      <strong>Улучшение текста</strong>
      <p>Включается по вашему выбору. Текст передаётся получателю, указанному при подключении.</p>
    </div>
    <span>По желанию</span>
  </div>
  <div class="fact">
    <Icon name="ph_key" />
    <div>
      <strong>Ключи и вход</strong>
      <p>{info.value?.keyStorage === 'keychain' ? 'Системная цепочка ключей macOS (Keychain)' : info.value?.keyStorage === 'dpapi' ? 'Защищённое хранилище Windows (DPAPI, текущий пользователь)' : 'Защищённое хранилище системы'}</p>
    </div>
  </div>
</section>
<section class="surface">
  <h2>Проект</h2>
  {#each [{ id: 'legal', title: 'Данные и условия', label: 'Открыть' }, { id: 'author', title: 'Дмитрий Киселев', label: '@amidexe' }, { id: 'source-v1', title: 'Исходный код версии 1', label: 'GitHub' }] as link}
    <div class="link-row">
      <span>{link.title}</span><button disabled={busy || info.failed} onclick={() => open(link.id)}
        >{link.label}<Icon name="ph_arrow_square_out" /></button
      >
    </div>
  {/each}
</section>
<section class="surface">
  <h2>Компоненты</h2>
  <p>GigaAM v3 · transcribe.cpp · Silero VAD</p>
  <p class="secondary">Модель и средства локального распознавания</p>
  <button
    disabled={!info.value}
    onclick={() => {
      licenses = true;
      error = false;
    }}><Icon name="ph_scroll" />Лицензии</button
  >
  {#if info.value}<details class="service">
      <summary>Сведения о движке</summary>
      <p>Версия {info.value.nativeVersion} ({info.value.platformBuild})</p>
      <p>Сборка {info.value.nativeBuildID || '—'}</p>
    </details>{/if}
</section>
<StatisticsReset />
{#if platform === 'macos'}<EraseLocalData />{/if}
{#if licenses && info.value}
  <Modal
    title="Лицензии"
    onclose={() => {
      licenses = false;
    }}
  >
    {#each [...uiLicenses, ...info.value.licenses] as license (license.id)}<div class="license">
        <strong>{license.name}</strong><span class="secondary">{license.spdx}</span>
        <p>{license.copyright}</p>
        <button class="quiet" disabled={busy || info.failed} onclick={() => open(license.id)}
          >Открыть проект<Icon name="ph_arrow_square_out" /></button
        >
      </div>{/each}
    {#if error}<p class="error" role="alert">Не удалось открыть ссылку. Попробуйте ещё раз.</p>{/if}
    {#snippet actions()}<button
        onclick={() => {
          licenses = false;
        }}>Закрыть</button
      >{/snippet}
  </Modal>
{/if}

<style>
  .surface + .surface {
    margin-top: 14px;
  }
  h2 {
    margin: 0 0 12px;
  }
  p {
    line-height: 1.5;
  }
  .secondary {
    color: var(--secondary);
  }
  .identity {
    display: flex;
    gap: 18px;
    align-items: center;
  }
  .identity p {
    margin: 5px 0 0;
  }
  .identity h2 {
    margin: 0;
    font-size: 22px;
  }
  .fact {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 12px 0;
  }
  .fact div {
    flex: 1;
  }
  .fact p {
    color: var(--secondary);
    margin: 5px 0 0;
  }
  .fact > span {
    color: var(--secondary);
    font-size: 11px;
  }
  .fact + .fact,
  .link-row + .link-row {
    border-top: 1px solid var(--stroke);
  }
  .link-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 10px;
    padding: 10px 0;
  }
  .license {
    padding: 12px 0;
    border-bottom: 1px solid var(--stroke);
  }
  .license > span {
    display: block;
    margin-top: 4px;
  }
  .license p {
    margin: 6px 0;
    font-size: 12px;
  }
  .service {
    margin-top: 16px;
    color: var(--secondary);
  }
  summary {
    cursor: pointer;
  }
</style>
