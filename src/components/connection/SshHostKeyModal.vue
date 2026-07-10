<script setup>
import BaseIcon from '../base/BaseIcon.vue'

// Driven entirely by App.vue: `prompt` is set for a first-contact trust request,
// `changed` for a refused connection whose host key no longer matches. At most
// one is non-null at a time; `prompt` wins if both somehow are.
const props = defineProps({
  prompt:  { type: Object, default: null },  // { requestId, host, port, fingerprint }
  changed: { type: Object, default: null },  // { host, port, storedFingerprint, presentedFingerprint }
})
const emit = defineEmits(['trust', 'cancel', 'forget', 'dismiss'])
</script>

<template>
  <!-- First-contact: ask the user to verify + trust the fingerprint. Backdrop /
       close act as Cancel so the waiting handshake always gets a decision. -->
  <div v-if="prompt" class="overlay" @mousedown.self="$emit('cancel')">
    <div class="dialog">
      <div class="dlg-title">
        <div class="t">Unknown SSH Host</div>
        <button class="close-btn" @click="$emit('cancel')">
          <BaseIcon name="close" :size="14" />
        </button>
      </div>

      <div class="hk-body">
        <div class="hk-lead">
          <span class="hk-ico"><BaseIcon name="lock" :size="22" /></span>
          <div>
            Connecting to <b>{{ prompt.host }}:{{ prompt.port }}</b> for the first time.
            The server presented this host key fingerprint:
          </div>
        </div>
        <div class="hk-fp">{{ prompt.fingerprint }}</div>
        <div class="hk-note">
          Only trust this host if the fingerprint matches the one your server
          administrator gave you. The key is saved and checked on every future
          connection.
        </div>
      </div>

      <div class="hk-footer">
        <span class="spacer"></span>
        <button class="btn" @click="$emit('cancel')">Cancel</button>
        <button class="btn primary" @click="$emit('trust')">Trust this host</button>
      </div>
    </div>
  </div>

  <!-- Key changed: the connection was already refused; explain and offer the
       deliberate recovery path (forget the saved key, then reconnect). -->
  <div v-else-if="changed" class="overlay" @mousedown.self="$emit('dismiss')">
    <div class="dialog">
      <div class="dlg-title">
        <div class="t danger">SSH Host Key Changed</div>
        <button class="close-btn" @click="$emit('dismiss')">
          <BaseIcon name="close" :size="14" />
        </button>
      </div>

      <div class="hk-body">
        <div class="hk-lead">
          <span class="hk-ico danger"><BaseIcon name="lock" :size="22" /></span>
          <div class="danger">
            <b>Warning:</b> the host key for <b>{{ changed.host }}:{{ changed.port }}</b>
            does not match the key previously trusted. This can mean a
            man-in-the-middle attack — or that the server's key was legitimately
            rotated. The connection was <b>refused</b>.
          </div>
        </div>
        <div class="hk-fp-row"><span class="hk-fp-label">Previously trusted</span>
          <div class="hk-fp">{{ changed.storedFingerprint }}</div></div>
        <div class="hk-fp-row"><span class="hk-fp-label">Now presented</span>
          <div class="hk-fp">{{ changed.presentedFingerprint }}</div></div>
        <div class="hk-note">
          If you are certain the key changed for a legitimate reason, forget the
          saved key — the next connection will ask you to verify and trust the
          new one.
        </div>
      </div>

      <div class="hk-footer">
        <span class="spacer"></span>
        <button class="btn" @click="$emit('dismiss')">Dismiss</button>
        <button class="btn danger" @click="$emit('forget')">Forget saved key</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, .5);
  display: grid;
  place-items: center;
  z-index: 70;
}

.dialog {
  width: 520px;
  max-width: 92vw;
  background: var(--bg-window);
  border-radius: 10px;
  box-shadow: 0 30px 80px rgba(0,0,0,.65), 0 0 0 1px var(--border);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.dlg-title {
  height: 36px;
  flex: none;
  background: linear-gradient(var(--dlg-titlebar-1), var(--dlg-titlebar-2));
  border-bottom: 1px solid var(--border);
  display: flex;
  align-items: center;
  padding: 0 10px;
  position: relative;
}
.dlg-title .t {
  position: absolute;
  left: 0; right: 0;
  text-align: center;
  font-size: 13px;
  color: var(--text-dim);
  font-weight: 500;
  pointer-events: none;
}
.dlg-title .t.danger { color: var(--danger-text); }

.close-btn {
  margin-left: auto;
  background: none;
  border: none;
  color: var(--text-faint);
  cursor: pointer;
  padding: 4px;
  display: flex;
  align-items: center;
  border-radius: 4px;
  z-index: 1;
}
.close-btn:hover { background: var(--bg-hover); color: var(--text); }

.hk-body {
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 12px;
  font-size: 13px;
  color: var(--text);
  line-height: 1.5;
}

.hk-lead {
  display: flex;
  gap: 12px;
  align-items: flex-start;
}
.hk-ico { color: var(--text-dim); flex: none; margin-top: 1px; }
.hk-ico.danger { color: var(--danger-text); }
.danger { color: var(--danger-text); }

.hk-fp {
  font-family: var(--mono);
  font-size: 12.5px;
  background: var(--bg-input);
  border: 1px solid var(--border);
  border-radius: 5px;
  padding: 8px 10px;
  color: var(--text);
  user-select: text;
  word-break: break-all;
}

.hk-fp-row { display: flex; flex-direction: column; gap: 4px; }
.hk-fp-label { font-size: 11px; color: var(--text-faint); text-transform: uppercase; letter-spacing: .04em; }

.hk-note { font-size: 12px; color: var(--text-dim); }

.hk-footer {
  height: 48px;
  flex: none;
  border-top: 1px solid var(--border);
  display: flex;
  align-items: center;
  padding: 0 16px;
  gap: 8px;
}
.spacer { flex: 1; }

.btn {
  height: 28px;
  padding: 0 14px;
  border-radius: 5px;
  border: none;
  font-size: 13px;
  cursor: pointer;
  background: var(--bg-toolbar);
  color: var(--text);
}
.btn:hover { background: var(--bg-hover); }
.btn.primary { background: var(--accent); color: #fff; }
.btn.primary:hover { opacity: .88; }
.btn.danger { background: var(--danger); color: #fff; }
.btn.danger:hover { opacity: .88; }
</style>
