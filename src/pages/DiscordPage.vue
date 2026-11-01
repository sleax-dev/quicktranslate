<script setup lang="ts">
import { computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { langName, t, uiLang } from '../i18n';
import { languages, page, settings } from '../store';
import Icon from '../components/Icon.vue';

const s = settings.value!;
/** Incoming messages go into "From", or the interface language while From detects (as on the Rust side). */
const yourLang = computed(() => langName(s.source_lang !== 'auto' ? s.source_lang : uiLang.value));
const named = computed(() => languages.value.map(([code, en]) => [code, langName(code, en)] as const));
</script>

<template>
  <section class="page">
    <header class="page-head">
      <div>
        <h1>{{ t('discord.title') }}</h1>
        <p>{{ t('discord.lead') }}</p>
      </div>
    </header>

    <div class="rows">
      <div class="row">
        <div class="info">
          <div class="name">{{ t('discord.send') }}</div>
          <div class="desc">{{ t('discord.send.desc') }}</div>
        </div>
        <button
          class="switch ctl"
          role="switch"
          :aria-checked="s.discord_send"
          :aria-label="t('discord.send')"
          @click="s.discord_send = !s.discord_send"
        />
      </div>
      <div class="row">
        <div class="info">
          <div class="name">{{ t('discord.read') }} <span class="badge">{{ t('discord.beta') }}</span></div>
          <div class="desc">{{ t('discord.read.desc') }}</div>
        </div>
        <button
          class="switch ctl"
          role="switch"
          :aria-checked="s.discord_read"
          :aria-label="t('discord.read')"
          @click="s.discord_read = !s.discord_read"
        />
      </div>
    </div>

    <section class="stack">
      <div class="sect-head"><h2>{{ t('discord.langs') }}</h2></div>
      <div class="rows">
        <div class="row">
          <div class="info">
            <div class="name">{{ t('discord.sendLang') }}</div>
            <div class="desc">{{ t('discord.sendLang.desc') }}</div>
          </div>
          <select v-model="s.discord_send_lang" class="select ctl" :aria-label="t('discord.sendLang')">
            <option value="">{{ t('discord.sendLang.default', { lang: langName(s.target_lang) }) }}</option>
            <option v-for="[code, name] in named" :key="code" :value="code">{{ name }}</option>
          </select>
        </div>
        <div class="row">
          <div class="info">
            <div class="name">{{ t('discord.incomingLang') }}</div>
            <div class="desc">{{ t('discord.incomingLang.desc') }}</div>
          </div>
          <select v-model="s.discord_incoming_lang" class="select ctl" :aria-label="t('discord.incomingLang')">
            <option value="">{{ t('discord.incomingLang.any') }}</option>
            <option v-for="[code, name] in named" :key="code" :value="code">{{ name }}</option>
          </select>
        </div>
        <div class="row">
          <div class="info">
            <div class="name">{{ t('discord.readLang') }}</div>
            <div class="desc">{{ t('discord.readLang.desc') }}</div>
          </div>
          <select v-model="s.discord_read_lang" class="select ctl" :aria-label="t('discord.readLang')">
            <option value="">{{ t('discord.readLang.default', { lang: yourLang }) }}</option>
            <option v-for="[code, name] in named" :key="code" :value="code">{{ name }}</option>
          </select>
        </div>
      </div>
    </section>

    <div class="rows">
      <div class="row">
        <div class="info">
          <div class="name">{{ t('words.title') }}</div>
          <div class="desc">{{ s.protected_words.length ? t('words.count', { n: s.protected_words.length }) : t('words.none') }}</div>
          <div v-if="s.protected_words.length" class="desc" style="margin-top: 4px">{{ s.protected_words.slice(0, 8).join(', ') }}{{ s.protected_words.length > 8 ? '…' : '' }}</div>
        </div>
        <button class="btn ctl" @click="page = 'settings'">{{ t('words.edit') }}</button>
      </div>
    </div>

    <div class="rows">
      <div class="row">
        <div class="info">
          <div class="name">{{ t('discord.report') }}</div>
          <div class="desc">{{ t('discord.report.desc') }}</div>
        </div>
        <button class="btn ctl" @click="invoke('discord_report')"><Icon name="report" />{{ t('discord.reportBtn') }}</button>
      </div>
    </div>
  </section>
</template>
