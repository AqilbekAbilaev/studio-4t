<script setup>
import { ref, computed, onMounted } from 'vue'
import BaseIcon from '../base/BaseIcon.vue'

// The Add / Edit index dialog (Screenshot A). It owns all of its form state and
// emits an assembled { keys, options } pair — two JSON strings the backend merges
// into one createIndexes spec. The JSON tab is the escape hatch for anything the
// form tabs don't cover yet (text/geo/collation land in a later step).
const props = defineProps({
  mode:  { type: String,  default: 'create' },   // 'create' | 'edit'
  seed:  { type: Object,  default: null },        // index spec to prefill (edit / paste)
  busy:  { type: Boolean, default: false },       // a create/edit request is in flight
  error: { type: String,  default: null },        // server-side error to surface
})
const emit = defineEmits(['submit', 'cancel'])

// Field type / direction options for each indexed field.
const TYPE_OPTIONS = [
  { value: '1',        label: '1 (asc)' },
  { value: '-1',       label: '-1 (desc)' },
  { value: '2dsphere', label: '2dsphere' },
  { value: '2d',       label: '2d' },
  { value: 'text',     label: 'text' },
  { value: 'hashed',   label: 'hashed' },
]

const name    = ref('')
const rows    = ref([{ field: '', type: '1' }])   // indexed fields, in order
const subtab  = ref('fields')                      // fields | options | text | geo | collation

// Options tab
const optUnique     = ref(false)
const optSparse     = ref(false)
const optHidden     = ref(false)
const optTtlEnabled = ref(false)
const optTtlSeconds = ref('')
const optPartial    = ref('')       // partialFilterExpression as JSON text

// Text options tab (text indexes)
const txtDefaultLang  = ref('')
const txtLangOverride = ref('')
const txtWeights      = ref('')     // field→weight map as JSON text

// Geo options tab (2dsphere / 2d indexes)
const geoSphereVersion = ref('')
const geoBits          = ref('')
const geoMin           = ref('')
const geoMax           = ref('')

// Collation tab
const colLocale         = ref('')
const colStrength       = ref('')   // '' | '1'..'5'
const colCaseLevel      = ref(false)
const colCaseFirst      = ref('off')
const colNumericOrdering = ref(false)
const colAlternate      = ref('non-ignorable')
const colBackwards      = ref(false)

// Footer
const background = ref(false)
const jsonMode   = ref(false)
const jsonText   = ref('')

const localError = ref(null)

// --- prefill from a seed (edit or paste) ---
onMounted(() => {
  const seed = props.seed
  if (!seed) return
  if (seed.key && typeof seed.key === 'object') {
    const seeded = Object.entries(seed.key).map(([field, value]) => ({ field: field, type: String(value) }))
    if (seeded.length) rows.value = seeded
  }
  // In edit mode the name is fixed to the original; paste starts unnamed.
  if (props.mode === 'edit') name.value = seed.name || ''
  optUnique.value = !!seed.unique
  optSparse.value = !!seed.sparse
  optHidden.value = !!seed.hidden
  if (seed.expireAfterSeconds != null) { optTtlEnabled.value = true; optTtlSeconds.value = String(seed.expireAfterSeconds) }
  if (seed.partialFilterExpression) optPartial.value = JSON.stringify(seed.partialFilterExpression)
  // text
  if (seed.default_language) txtDefaultLang.value = seed.default_language
  if (seed.language_override) txtLangOverride.value = seed.language_override
  if (seed.weights) txtWeights.value = JSON.stringify(seed.weights)
  // geo
  if (seed['2dsphereIndexVersion'] != null) geoSphereVersion.value = String(seed['2dsphereIndexVersion'])
  if (seed.bits != null) geoBits.value = String(seed.bits)
  if (seed.min != null) geoMin.value = String(seed.min)
  if (seed.max != null) geoMax.value = String(seed.max)
  // collation
  const col = seed.collation
  if (col && typeof col === 'object') {
    colLocale.value = col.locale || ''
    if (col.strength != null) colStrength.value = String(col.strength)
    colCaseLevel.value = !!col.caseLevel
    colCaseFirst.value = col.caseFirst || 'off'
    colNumericOrdering.value = !!col.numericOrdering
    colAlternate.value = col.alternate || 'non-ignorable'
    colBackwards.value = !!col.backwards
  }
})

// --- fields ---
function addField() { rows.value.push({ field: '', type: '1' }) }
function removeField(i) { rows.value.splice(i, 1); if (!rows.value.length) addField() }

const fieldCount = computed(() => rows.value.filter(r => r.field.trim()).length)
const kindLabel = computed(() => fieldCount.value > 1 ? 'Compound index' : (fieldCount.value === 1 ? 'Single-field index' : 'No fields yet'))

// --- spec assembly ---
function mapType(t) {
  if (t === '1') return 1
  if (t === '-1') return -1
  return t
}
function buildKeys() {
  const key = {}
  for (const row of rows.value) {
    const field = row.field.trim()
    if (field) key[field] = mapType(row.type)
  }
  return key
}
// Throws with a user-facing message on invalid input (bad TTL / partial filter).
function buildOptions() {
  const options = {}
  if (name.value.trim()) options.name = name.value.trim()
  if (optUnique.value) options.unique = true
  if (optSparse.value) options.sparse = true
  if (optHidden.value) options.hidden = true
  if (background.value) options.background = true
  if (optTtlEnabled.value) {
    const secs = Number(optTtlSeconds.value)
    if (!Number.isFinite(secs) || secs < 0) throw new Error('TTL seconds must be a non-negative number')
    options.expireAfterSeconds = secs
  }
  if (optPartial.value.trim()) {
    try { options.partialFilterExpression = JSON.parse(optPartial.value) }
    catch (e) { throw new Error('Partial filter expression is not valid JSON') }
  }
  // Text options
  if (txtDefaultLang.value.trim()) options.default_language = txtDefaultLang.value.trim()
  if (txtLangOverride.value.trim()) options.language_override = txtLangOverride.value.trim()
  if (txtWeights.value.trim()) {
    try { options.weights = JSON.parse(txtWeights.value) }
    catch (e) { throw new Error('Text weights must be valid JSON') }
  }
  // Geo options
  if (geoSphereVersion.value !== '') options['2dsphereIndexVersion'] = numOrThrow(geoSphereVersion.value, '2dsphere index version')
  if (geoBits.value !== '') options.bits = numOrThrow(geoBits.value, 'bits')
  if (geoMin.value !== '') options.min = numOrThrow(geoMin.value, 'min')
  if (geoMax.value !== '') options.max = numOrThrow(geoMax.value, 'max')
  // Collation (only when a locale is given)
  if (colLocale.value.trim()) {
    const collation = { locale: colLocale.value.trim() }
    if (colStrength.value) collation.strength = Number(colStrength.value)
    if (colCaseLevel.value) collation.caseLevel = true
    if (colCaseFirst.value && colCaseFirst.value !== 'off') collation.caseFirst = colCaseFirst.value
    if (colNumericOrdering.value) collation.numericOrdering = true
    if (colAlternate.value && colAlternate.value !== 'non-ignorable') collation.alternate = colAlternate.value
    if (colBackwards.value) collation.backwards = true
    options.collation = collation
  }
  return options
}
function numOrThrow(value, label) {
  const n = Number(value)
  if (!Number.isFinite(n)) throw new Error(`${label} must be a number`)
  return n
}

// The assembled spec ({ key, ...options }) shown/edited in JSON mode.
function currentSpec() {
  const spec = { key: buildKeys() }
  Object.assign(spec, buildOptionsSafe())
  return spec
}
function buildOptionsSafe() { try { return buildOptions() } catch (e) { return {} } }

function toggleJson() {
  if (!jsonMode.value) {
    jsonText.value = JSON.stringify(currentSpec(), null, 2)
    jsonMode.value = true
  } else {
    jsonMode.value = false
  }
  localError.value = null
}

function onSubmit() {
  localError.value = null
  if (jsonMode.value) {
    let spec
    try { spec = JSON.parse(jsonText.value) } catch (e) { localError.value = 'Index spec is not valid JSON'; return }
    if (!spec || typeof spec.key !== 'object' || !Object.keys(spec.key).length) {
      localError.value = 'The spec needs a non-empty "key" object'; return
    }
    const key = spec.key
    const options = Object.assign({}, spec)
    delete options.key
    emit('submit', { keys: JSON.stringify(key), options: JSON.stringify(options) })
    return
  }
  const key = buildKeys()
  if (!Object.keys(key).length) { localError.value = 'Add at least one field'; return }
  let options
  try { options = buildOptions() } catch (e) { localError.value = e.message; return }
  emit('submit', { keys: JSON.stringify(key), options: JSON.stringify(options) })
}

const shownError = computed(() => localError.value || props.error)
const title = computed(() => props.mode === 'edit' ? 'Edit index' : 'Add index')
</script>

<template>
  <div class="del-overlay" @mousedown.self="emit('cancel')">
    <div class="del-dialog idx-add-dialog">
      <div class="del-title">
        <div class="t">{{ title }}</div>
        <button class="close-btn" @click="emit('cancel')"><BaseIcon name="close" :size="14" /></button>
      </div>

      <div class="del-body idx-add-body">
        <label class="idx-flabel">Index name</label>
        <input
          v-model="name"
          class="prompt-input"
          :disabled="mode === 'edit'"
          placeholder="Auto-generated from the fields if left blank"
          spellcheck="false" autocorrect="off" autocapitalize="off"
        />

        <!-- Sub-tabs -->
        <div class="sub-tabs">
          <button class="sub-tab" :class="{ active: subtab === 'fields' }" :disabled="jsonMode" @click="subtab = 'fields'">Fields</button>
          <button class="sub-tab" :class="{ active: subtab === 'options' }" :disabled="jsonMode" @click="subtab = 'options'">Options</button>
          <button class="sub-tab" :class="{ active: subtab === 'text' }" :disabled="jsonMode" @click="subtab = 'text'">Text options</button>
          <button class="sub-tab" :class="{ active: subtab === 'geo' }" :disabled="jsonMode" @click="subtab = 'geo'">Geo options</button>
          <button class="sub-tab" :class="{ active: subtab === 'collation' }" :disabled="jsonMode" @click="subtab = 'collation'">Collation</button>
          <span v-if="jsonMode" class="json-badge">Editing raw JSON</span>
        </div>

        <!-- JSON mode -->
        <div v-if="jsonMode" class="json-pane">
          <textarea
            v-model="jsonText"
            class="json-area"
            spellcheck="false" autocorrect="off" autocapitalize="off"
          ></textarea>
        </div>

        <!-- Fields tab -->
        <div v-else-if="subtab === 'fields'" class="tab-pane">
          <table class="fields-table">
            <thead>
              <tr><th class="fc-name">Field name</th><th class="fc-type">Field type</th><th class="fc-x"></th></tr>
            </thead>
            <tbody>
              <tr v-for="(row, i) in rows" :key="i">
                <td class="fc-name">
                  <input v-model="row.field" class="prompt-input sm" placeholder="e.g. email or address.city" spellcheck="false" autocorrect="off" autocapitalize="off" />
                </td>
                <td class="fc-type">
                  <select v-model="row.type" class="prompt-input sm">
                    <option v-for="opt in TYPE_OPTIONS" :key="opt.value" :value="opt.value">{{ opt.label }}</option>
                  </select>
                </td>
                <td class="fc-x">
                  <button class="icon-btn" title="Remove field" @click="removeField(i)"><BaseIcon name="trash" :size="14" /></button>
                </td>
              </tr>
            </tbody>
          </table>
          <div class="fields-foot">
            <button class="btn sm" @click="addField"><BaseIcon name="plus" :size="13" /> Add field</button>
            <span class="kind-label">{{ kindLabel }}</span>
          </div>
        </div>

        <!-- Options tab -->
        <div v-else-if="subtab === 'options'" class="tab-pane options-pane">
          <label class="opt-row"><input type="checkbox" v-model="optUnique" /><span>Unique</span></label>
          <label class="opt-row"><input type="checkbox" v-model="optSparse" /><span>Sparse</span></label>
          <label class="opt-row"><input type="checkbox" v-model="optHidden" /><span>Hidden (ignored by the query planner)</span></label>
          <label class="opt-row"><input type="checkbox" v-model="optTtlEnabled" /><span>TTL — expire documents after</span>
            <input v-model="optTtlSeconds" class="prompt-input sm ttl" :disabled="!optTtlEnabled" placeholder="seconds" /></label>
          <label class="idx-flabel">Partial filter expression (JSON)</label>
          <input v-model="optPartial" class="prompt-input" placeholder='e.g. {"status": "active"}' spellcheck="false" autocorrect="off" autocapitalize="off" />
        </div>

        <!-- Text options tab -->
        <div v-else-if="subtab === 'text'" class="tab-pane">
          <p class="pane-note">Applies to <code>text</code> indexes.</p>
          <label class="idx-flabel">Default language</label>
          <input v-model="txtDefaultLang" class="prompt-input" placeholder="english" spellcheck="false" autocorrect="off" autocapitalize="off" />
          <label class="idx-flabel">Language override field</label>
          <input v-model="txtLangOverride" class="prompt-input" placeholder="language" spellcheck="false" autocorrect="off" autocapitalize="off" />
          <label class="idx-flabel">Field weights (JSON)</label>
          <input v-model="txtWeights" class="prompt-input" placeholder='e.g. {"title": 10, "body": 1}' spellcheck="false" autocorrect="off" autocapitalize="off" />
        </div>

        <!-- Geo options tab -->
        <div v-else-if="subtab === 'geo'" class="tab-pane">
          <p class="pane-note">Applies to <code>2dsphere</code> / <code>2d</code> indexes.</p>
          <label class="idx-flabel">2dsphere index version</label>
          <input v-model="geoSphereVersion" class="prompt-input" placeholder="3" spellcheck="false" autocorrect="off" autocapitalize="off" />
          <div class="geo-grid">
            <div>
              <label class="idx-flabel">Bits (2d)</label>
              <input v-model="geoBits" class="prompt-input" placeholder="26" spellcheck="false" autocorrect="off" autocapitalize="off" />
            </div>
            <div>
              <label class="idx-flabel">Min (2d)</label>
              <input v-model="geoMin" class="prompt-input" placeholder="-180" spellcheck="false" autocorrect="off" autocapitalize="off" />
            </div>
            <div>
              <label class="idx-flabel">Max (2d)</label>
              <input v-model="geoMax" class="prompt-input" placeholder="180" spellcheck="false" autocorrect="off" autocapitalize="off" />
            </div>
          </div>
        </div>

        <!-- Collation tab -->
        <div v-else class="tab-pane options-pane">
          <p class="pane-note">Set a locale to attach a collation; leave blank for none.</p>
          <label class="idx-flabel">Locale</label>
          <input v-model="colLocale" class="prompt-input" placeholder='e.g. en or "simple"' spellcheck="false" autocorrect="off" autocapitalize="off" />
          <div class="geo-grid">
            <div>
              <label class="idx-flabel">Strength</label>
              <select v-model="colStrength" class="prompt-input">
                <option value="">default</option>
                <option value="1">1 — primary</option>
                <option value="2">2 — secondary</option>
                <option value="3">3 — tertiary</option>
                <option value="4">4 — quaternary</option>
                <option value="5">5 — identical</option>
              </select>
            </div>
            <div>
              <label class="idx-flabel">Case first</label>
              <select v-model="colCaseFirst" class="prompt-input">
                <option value="off">off</option>
                <option value="upper">upper</option>
                <option value="lower">lower</option>
              </select>
            </div>
            <div>
              <label class="idx-flabel">Alternate</label>
              <select v-model="colAlternate" class="prompt-input">
                <option value="non-ignorable">non-ignorable</option>
                <option value="shifted">shifted</option>
              </select>
            </div>
          </div>
          <label class="opt-row"><input type="checkbox" v-model="colCaseLevel" /><span>Case level</span></label>
          <label class="opt-row"><input type="checkbox" v-model="colNumericOrdering" /><span>Numeric ordering</span></label>
          <label class="opt-row"><input type="checkbox" v-model="colBackwards" /><span>Backwards (French accent sort)</span></label>
        </div>

        <div v-if="shownError" class="del-error">{{ shownError }}</div>
      </div>

      <div class="del-footer idx-add-footer">
        <label class="bg-check"><input type="checkbox" v-model="background" /><span>Create in background</span></label>
        <button class="btn sm json-btn" @click="toggleJson">{{ jsonMode ? 'Form' : 'JSON' }}</button>
        <span class="spacer"></span>
        <button class="btn" @click="emit('cancel')">Cancel</button>
        <button class="btn primary" :disabled="busy" @click="onSubmit">
          {{ busy ? (mode === 'edit' ? 'Saving…' : 'Creating…') : (mode === 'edit' ? 'Save changes' : 'Create index') }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.idx-add-dialog { width: 560px; max-width: 92vw; }
.idx-add-body { display: flex; flex-direction: column; }
.idx-flabel { display: block; font-size: 12px; color: var(--text-dim); margin: 12px 0 4px; }
.idx-flabel:first-child { margin-top: 0; }

.sub-tabs { display: flex; align-items: center; gap: 2px; margin: 14px 0 10px; border-bottom: 1px solid var(--border); }
.sub-tab {
  border: none; background: transparent; color: var(--text-dim);
  padding: 6px 12px; font-size: 12.5px; cursor: pointer;
  border-bottom: 2px solid transparent; margin-bottom: -1px;
}
.sub-tab:hover:not(:disabled) { color: var(--text); }
.sub-tab.active { color: var(--text); border-bottom-color: var(--accent); }
.sub-tab:disabled { color: var(--text-faint); cursor: default; }
.json-badge { margin-left: auto; font-size: 11.5px; color: var(--text-faint); padding-bottom: 6px; }

.tab-pane { min-height: 180px; }

.fields-table { width: 100%; border-collapse: collapse; font-size: 12.5px; }
.fields-table th {
  text-align: left; font-weight: 600; color: var(--text-dim);
  padding: 4px 6px; border-bottom: 1px solid var(--border);
}
.fields-table td { padding: 3px 6px; }
.fc-type { width: 150px; }
.fc-x { width: 34px; text-align: center; }
.prompt-input.sm { padding: 4px 7px; font-size: 12.5px; width: 100%; }
.prompt-input.ttl { width: 110px; margin-left: 8px; }
.icon-btn { border: none; background: transparent; color: var(--text-faint); cursor: pointer; padding: 3px; border-radius: var(--radius); display: inline-flex; }
.icon-btn:hover { color: var(--danger-text); background: var(--bg-hover); }

.fields-foot { display: flex; align-items: center; gap: 12px; margin-top: 10px; }
.btn.sm { padding: 4px 9px; font-size: 12px; display: inline-flex; align-items: center; gap: 5px; }
.kind-label { font-size: 12px; color: var(--text-faint); }

.options-pane { display: flex; flex-direction: column; gap: 4px; }
.opt-row { display: flex; align-items: center; gap: 8px; font-size: 12.5px; color: var(--text); padding: 4px 0; }
.pane-note { font-size: 12px; color: var(--text-faint); margin: 0 0 4px; }
.pane-note code { font-family: var(--mono); font-size: 11.5px; }
.geo-grid { display: grid; grid-template-columns: repeat(3, 1fr); gap: 6px 12px; }
.geo-grid select.prompt-input { width: 100%; }

.json-pane { min-height: 180px; }
.json-area {
  width: 100%; min-height: 200px; resize: vertical;
  font-family: var(--mono); font-size: 12.5px; line-height: 1.5;
  padding: 8px 10px; border-radius: var(--radius);
  background: var(--bg-input); color: var(--text); border: 1px solid var(--border);
}

.idx-add-footer { display: flex; align-items: center; gap: 12px; }
.bg-check { display: flex; align-items: center; gap: 7px; font-size: 12.5px; color: var(--text); }
.json-btn { margin-left: 4px; }
</style>

<!-- Shared dialog/button classes (.del-*, .btn, .prompt-input, …) from App.css,
     imported scoped like AppModals/IndexManagerPane do. -->
<style src="../../App.css" scoped></style>
