<script setup>
import { inject } from 'vue'
import BaseIcon from '../base/BaseIcon.vue'
import { indexKeyLabel, indexSpecJson, isIndexHidden } from '../../utils/indexSpec'
import ConnectionManager from '../connection/ConnectionManager.vue'
import ServerStatusModal from '../admin/ServerStatusModal.vue'
import DatabaseStatsModal from '../admin/DatabaseStatsModal.vue'
import CurrentOpsModal from '../admin/CurrentOpsModal.vue'
import ProfilerModal from '../admin/ProfilerModal.vue'
import ValidatorModal from '../admin/ValidatorModal.vue'
import UsersModal from '../admin/UsersModal.vue'
import RolesModal from '../admin/RolesModal.vue'
import FunctionsModal from '../admin/FunctionsModal.vue'
import MapReduceModal from '../query/MapReduceModal.vue'
import ServerChartsModal from '../admin/ServerChartsModal.vue'
import SchemaModal from '../tools/SchemaModal.vue'
import CollectionHistoryModal from '../tools/CollectionHistoryModal.vue'
import SqlModal from '../query/SqlModal.vue'
import TasksModal from '../admin/TasksModal.vue'
import MaskingModal from '../tools/MaskingModal.vue'
import ImportExportWizard from '../tools/ImportExportWizard.vue'
import ReschemaModal from '../tools/ReschemaModal.vue'
import StatsModal from '../admin/StatsModal.vue'
import ServerInfoModal from '../admin/ServerInfoModal.vue'
import MigrationModal from '../tools/MigrationModal.vue'
import SearchModal from '../tools/SearchModal.vue'
import GridFsModal from '../tools/GridFsModal.vue'
import CompareModal from '../tools/CompareModal.vue'
import ShortcutsModal from './ShortcutsModal.vue'
import AboutModal from './AboutModal.vue'
import PreferencesModal from './PreferencesModal.vue'
import SshHostKeyModal from '../connection/SshHostKeyModal.vue'

// Single provide/inject from App.vue. Each group is destructured back to the same
// identifier names the moved template already uses, so that template is verbatim.
const ctx = inject('appModals')

const {
  showConnectionManager,
  serverStatusTarget,
  dbStatsTarget,
  currentOpsTarget,
  profilerTarget,
  validatorTarget,
  usersTarget,
  rolesTarget,
  functionsTarget,
  mapReduceTarget,
  serverChartsTarget,
  migrationTarget,
  searchTarget,
  gridfsTarget,
  gridfsRequest,
  compareTarget,
  schemaTarget,
  historyTarget,
  showSqlModal,
  showTasksModal,
  maskingTarget,
  importWizardTarget,
  exportWizardTarget,
  reschemaTarget,
  statsTarget,
  serverInfoTarget,
  showShortcuts,
  showAbout,
  showPreferences,
} = ctx.modals

const {
  indexesTarget,
  indexesList,
  indexesLoading,
  indexesError,
  selectedIndex,
  pendingDropIndex,
  newIndexKeys,
  newIndexName,
  newIndexUnique,
  indexCreating,
  indexFormMode,
  indexDetailsTarget,
  indexDetailsStats,
  indexDetailsLoading,
  dropIndexTarget,
  dropIndexConfirmText,
  dropIndexError,
  dropIndexBusy,
  closeIndexesModal,
  dropIndex,
  confirmCreateIndex,
  resetIndexForm,
  confirmDropIndex,
  formatIndexSince,
} = ctx.indexes

const {
  addCollectionTarget,
  newCollectionName,
  newCollectionType,
  newCollectionOpts,
  addCollectionError,
  addCollectionSaving,
  addViewTarget,
  newViewName,
  newViewSource,
  newViewPipeline,
  addViewError,
  addViewSaving,
  addBucketTarget,
  newBucketName,
  addBucketError,
  addBucketSaving,
  dropDatabaseTarget,
  dropDatabaseError,
  dropDatabaseDeleting,
  dropCollectionTarget,
  dropCollectionError,
  dropCollectionDeleting,
  renameCollectionTarget,
  renameCollectionName,
  renameCollectionError,
  renameCollectionSaving,
  duplicateCollectionTarget,
  duplicateCollectionName,
  duplicateCollectionError,
  duplicateCollectionSaving,
  addDatabaseTarget,
  newDatabaseName,
  newDatabaseCollName,
  addDatabaseError,
  addDatabaseSaving,
  confirmAddCollection,
  confirmAddView,
  confirmAddBucket,
  confirmDropDatabase,
  confirmDropCollection,
  confirmRenameCollection,
  confirmDuplicateCollection,
  confirmAddDatabase,
} = ctx.dbActions

const {
  sshHostKeyPrompt,
  sshHostKeyChanged,
  onHostKeyTrust,
  onHostKeyCancel,
  onHostKeyForget,
} = ctx.ssh

const {
  showToast,
  onManagerConnect,
  onValidatorSaved,
  onWizardImported,
  onReschemaApplied,
  onPrefsSaved,
  onKeybindingsSaved,
} = ctx.handlers

const { defaultQueryLimit, theme, keyBindings } = ctx.prefs

const { renameTabTarget, renameTabValue, confirmRenameTab } = ctx.tabRename
</script>

<template>
    <!-- Connection Manager modal -->
    <ConnectionManager
      v-if="showConnectionManager"
      @close="showConnectionManager = false"
      @connect="onManagerConnect"
      @toast="showToast"
    />

    <!-- Server Status modal -->
    <ServerStatusModal
      v-if="serverStatusTarget"
      :target="serverStatusTarget"
      @close="serverStatusTarget = null"
    />

    <DatabaseStatsModal
      v-if="dbStatsTarget"
      :target="dbStatsTarget"
      @close="dbStatsTarget = null"
    />

    <CurrentOpsModal
      v-if="currentOpsTarget"
      :target="currentOpsTarget"
      @close="currentOpsTarget = null"
    />

    <ProfilerModal
      v-if="profilerTarget"
      :target="profilerTarget"
      @close="profilerTarget = null"
    />

    <ValidatorModal
      v-if="validatorTarget"
      :target="validatorTarget"
      @saved="onValidatorSaved"
      @close="validatorTarget = null"
    />

    <UsersModal
      v-if="usersTarget"
      :target="usersTarget"
      @close="usersTarget = null"
    />

    <RolesModal
      v-if="rolesTarget"
      :target="rolesTarget"
      @close="rolesTarget = null"
    />

    <FunctionsModal
      v-if="functionsTarget"
      :target="functionsTarget"
      @close="functionsTarget = null"
    />

    <MapReduceModal
      v-if="mapReduceTarget"
      :target="mapReduceTarget"
      @close="mapReduceTarget = null"
    />

    <ServerChartsModal
      v-if="serverChartsTarget"
      :target="serverChartsTarget"
      @close="serverChartsTarget = null"
    />

    <!-- Schema (View Schema) modal -->
    <SchemaModal
      v-if="schemaTarget"
      :target="schemaTarget"
      @close="schemaTarget = null"
    />

    <!-- Collection History modal -->
    <CollectionHistoryModal
      v-if="historyTarget"
      :target="historyTarget"
      @close="historyTarget = null"
    />

    <!-- SQL → MQL translator -->
    <SqlModal
      v-if="showSqlModal"
      @close="showSqlModal = false"
    />

    <!-- Tasks panel -->
    <TasksModal
      v-if="showTasksModal"
      @toast="showToast"
      @close="showTasksModal = false"
    />

    <!-- Data Masking modal -->
    <MaskingModal
      v-if="maskingTarget"
      :target="maskingTarget"
      @toast="showToast"
      @close="maskingTarget = null"
    />

    <!-- Import / Export field-mapping wizard -->
    <ImportExportWizard
      v-if="importWizardTarget"
      mode="import"
      :target="importWizardTarget"
      @toast="showToast"
      @done="onWizardImported"
      @close="importWizardTarget = null"
    />
    <ImportExportWizard
      v-if="exportWizardTarget"
      mode="export"
      :target="exportWizardTarget"
      @toast="showToast"
      @close="exportWizardTarget = null"
    />

    <!-- Reschema modal -->
    <ReschemaModal
      v-if="reschemaTarget"
      :target="reschemaTarget"
      @toast="showToast"
      @applied="onReschemaApplied"
      @close="reschemaTarget = null"
    />

    <!-- Collection Stats modal -->
    <StatsModal
      v-if="statsTarget"
      :target="statsTarget"
      @close="statsTarget = null"
    />

    <!-- Build / Host / Replica Set info modal -->
    <ServerInfoModal
      v-if="serverInfoTarget"
      :target="serverInfoTarget"
      @close="serverInfoTarget = null"
    />

    <!-- SQL Migration modal -->
    <MigrationModal
      v-if="migrationTarget"
      :target="migrationTarget"
      @close="migrationTarget = null"
    />

    <!-- Global Search modal -->
    <SearchModal
      v-if="searchTarget"
      :target="searchTarget"
      @close="searchTarget = null"
    />

    <!-- GridFS modal -->
    <GridFsModal
      v-if="gridfsTarget"
      :target="gridfsTarget"
      :menu-request="gridfsRequest"
      @toast="showToast"
      @close="gridfsTarget = null"
    />

    <!-- Data Compare modal -->
    <CompareModal
      v-if="compareTarget"
      :target="compareTarget"
      @close="compareTarget = null"
    />

    <!-- Keyboard Shortcuts (customizable) -->
    <ShortcutsModal
      v-if="showShortcuts"
      :bindings="keyBindings"
      @save="onKeybindingsSaved"
      @close="showShortcuts = false"
    />

    <!-- About -->
    <AboutModal
      v-if="showAbout"
      @close="showAbout = false"
    />

    <!-- Preferences -->
    <PreferencesModal
      v-if="showPreferences"
      :default-query-limit="defaultQueryLimit"
      :theme="theme"
      @close="showPreferences = false"
      @saved="onPrefsSaved"
      @open-shortcuts="showPreferences = false; showShortcuts = true"
    />

    <!-- SSH host-key trust prompt / changed-key warning -->
    <SshHostKeyModal
      :prompt="sshHostKeyPrompt"
      :changed="sshHostKeyChanged"
      @trust="onHostKeyTrust"
      @cancel="onHostKeyCancel"
      @forget="onHostKeyForget"
      @dismiss="sshHostKeyChanged = null"
    />

    <!-- Add Collection modal -->
    <div v-if="addCollectionTarget" class="del-overlay" @mousedown.self="addCollectionTarget = null">
      <div class="del-dialog">
        <div class="del-title">
          <div class="t">Add Collection</div>
          <button class="close-btn" @click="addCollectionTarget = null">
            <BaseIcon name="close" :size="14" />
          </button>
        </div>
        <div class="del-body">
          <input
            v-model="newCollectionName"
            class="prompt-input"
            placeholder="Collection name"
            spellcheck="false"
            autocorrect="off"
            autocapitalize="off"
            @keydown.enter="confirmAddCollection"
          />
          <div class="cc-types">
            <label class="cc-type"><input type="radio" value="standard" v-model="newCollectionType" /> Standard</label>
            <label class="cc-type"><input type="radio" value="capped" v-model="newCollectionType" /> Capped</label>
            <label class="cc-type"><input type="radio" value="timeseries" v-model="newCollectionType" /> Time-series</label>
            <label class="cc-type"><input type="radio" value="clustered" v-model="newCollectionType" /> Clustered</label>
          </div>

          <div v-if="newCollectionType === 'capped'" class="cc-opts">
            <label class="cc-field">
              <span class="cc-label">Max size (bytes)</span>
              <input v-model="newCollectionOpts.size" class="prompt-input" type="number" min="1" placeholder="e.g. 1048576" @keydown.enter="confirmAddCollection" />
            </label>
            <label class="cc-field">
              <span class="cc-label">Max documents <span class="cc-opt">(optional)</span></span>
              <input v-model="newCollectionOpts.max" class="prompt-input" type="number" min="1" placeholder="e.g. 1000" @keydown.enter="confirmAddCollection" />
            </label>
          </div>

          <div v-else-if="newCollectionType === 'timeseries'" class="cc-opts">
            <label class="cc-field">
              <span class="cc-label">Time field</span>
              <input v-model="newCollectionOpts.timeField" class="prompt-input" spellcheck="false" autocorrect="off" autocapitalize="off" placeholder="e.g. timestamp" @keydown.enter="confirmAddCollection" />
            </label>
            <label class="cc-field">
              <span class="cc-label">Meta field <span class="cc-opt">(optional)</span></span>
              <input v-model="newCollectionOpts.metaField" class="prompt-input" spellcheck="false" autocorrect="off" autocapitalize="off" placeholder="e.g. metadata" @keydown.enter="confirmAddCollection" />
            </label>
            <label class="cc-field">
              <span class="cc-label">Granularity <span class="cc-opt">(optional)</span></span>
              <select v-model="newCollectionOpts.granularity" class="prompt-input">
                <option value="">Auto</option>
                <option value="seconds">Seconds</option>
                <option value="minutes">Minutes</option>
                <option value="hours">Hours</option>
              </select>
            </label>
            <label class="cc-field">
              <span class="cc-label">Expire after (seconds) <span class="cc-opt">(optional)</span></span>
              <input v-model="newCollectionOpts.expireAfterSeconds" class="prompt-input" type="number" min="1" placeholder="e.g. 86400" @keydown.enter="confirmAddCollection" />
            </label>
          </div>

          <div v-else-if="newCollectionType === 'clustered'" class="cc-opts">
            <p class="cc-hint">Documents are stored in <code>_id</code> order (clustered index on <code>{ _id: 1 }</code>).</p>
            <label class="cc-field">
              <span class="cc-label">Index name <span class="cc-opt">(optional)</span></span>
              <input v-model="newCollectionOpts.clusteredIndexName" class="prompt-input" spellcheck="false" autocorrect="off" autocapitalize="off" placeholder="e.g. events_clustered" @keydown.enter="confirmAddCollection" />
            </label>
          </div>

          <div v-if="addCollectionError" class="del-error">{{ addCollectionError }}</div>
        </div>
        <div class="del-footer">
          <span class="spacer"></span>
          <button class="btn" @click="addCollectionTarget = null">Cancel</button>
          <button class="btn primary" :disabled="!newCollectionName.trim() || addCollectionSaving" @click="confirmAddCollection">
            {{ addCollectionSaving ? 'Creating…' : 'Create' }}
          </button>
        </div>
      </div>
    </div>

    <!-- Add View modal -->
    <div v-if="addViewTarget" class="del-overlay" @mousedown.self="addViewTarget = null">
      <div class="del-dialog">
        <div class="del-title">
          <div class="t">Add View</div>
          <button class="close-btn" @click="addViewTarget = null">
            <BaseIcon name="close" :size="14" />
          </button>
        </div>
        <div class="del-body">
          <input
            v-model="newViewName"
            class="prompt-input"
            placeholder="View name"
            spellcheck="false"
            autocorrect="off"
            autocapitalize="off"
          />
          <input
            v-model="newViewSource"
            class="prompt-input"
            placeholder="Source collection (viewOn)"
            spellcheck="false"
            autocorrect="off"
            autocapitalize="off"
          />
          <textarea
            v-model="newViewPipeline"
            class="prompt-input pipeline-input"
            placeholder="Aggregation pipeline (optional), e.g. [ { &quot;$match&quot;: { &quot;active&quot;: true } } ]"
            spellcheck="false"
          ></textarea>
          <div v-if="addViewError" class="del-error">{{ addViewError }}</div>
        </div>
        <div class="del-footer">
          <span class="spacer"></span>
          <button class="btn" @click="addViewTarget = null">Cancel</button>
          <button class="btn primary" :disabled="!newViewName.trim() || !newViewSource.trim() || addViewSaving" @click="confirmAddView">
            {{ addViewSaving ? 'Creating…' : 'Create' }}
          </button>
        </div>
      </div>
    </div>

    <!-- Add GridFS Bucket modal -->
    <div v-if="addBucketTarget" class="del-overlay" @mousedown.self="addBucketTarget = null">
      <div class="del-dialog">
        <div class="del-title">
          <div class="t">Add GridFS Bucket</div>
          <button class="close-btn" @click="addBucketTarget = null">
            <BaseIcon name="close" :size="14" />
          </button>
        </div>
        <div class="del-body">
          <input
            v-model="newBucketName"
            class="prompt-input"
            placeholder="Bucket name (e.g. fs)"
            spellcheck="false"
            autocorrect="off"
            autocapitalize="off"
            @keydown.enter="confirmAddBucket"
          />
          <div v-if="addBucketError" class="del-error">{{ addBucketError }}</div>
        </div>
        <div class="del-footer">
          <span class="spacer"></span>
          <button class="btn" @click="addBucketTarget = null">Cancel</button>
          <button class="btn primary" :disabled="!newBucketName.trim() || addBucketSaving" @click="confirmAddBucket">
            {{ addBucketSaving ? 'Creating…' : 'Create' }}
          </button>
        </div>
      </div>
    </div>

    <!-- Rename Tab modal -->
    <div v-if="renameTabTarget" class="del-overlay" @mousedown.self="renameTabTarget = null">
      <div class="del-dialog">
        <div class="del-title">
          <div class="t">Rename Tab</div>
          <button class="close-btn" @click="renameTabTarget = null">
            <BaseIcon name="close" :size="14" />
          </button>
        </div>
        <div class="del-body">
          <input
            v-model="renameTabValue"
            class="prompt-input"
            placeholder="Tab name"
            spellcheck="false"
            autocorrect="off"
            autocapitalize="off"
            @keydown.enter="confirmRenameTab"
            @keydown.escape="renameTabTarget = null"
          />
        </div>
        <div class="del-footer">
          <span class="spacer"></span>
          <button class="btn" @click="renameTabTarget = null">Cancel</button>
          <button class="btn primary" :disabled="!renameTabValue.trim()" @click="confirmRenameTab">Rename</button>
        </div>
      </div>
    </div>

    <!-- Drop Database confirm -->
    <div v-if="dropDatabaseTarget" class="del-overlay" @mousedown.self="dropDatabaseTarget = null">
      <div class="del-dialog">
        <div class="del-title">
          <div class="t">Drop Database</div>
          <button class="close-btn" @click="dropDatabaseTarget = null">
            <BaseIcon name="close" :size="14" />
          </button>
        </div>
        <div class="del-body">
          <p>Are you sure you want to drop "<strong>{{ dropDatabaseTarget.dbName }}</strong>"? This deletes all of its collections and cannot be undone.</p>
          <div v-if="dropDatabaseError" class="del-error">{{ dropDatabaseError }}</div>
        </div>
        <div class="del-footer">
          <span class="spacer"></span>
          <button class="btn" @click="dropDatabaseTarget = null">Cancel</button>
          <button class="btn danger" :disabled="dropDatabaseDeleting" @click="confirmDropDatabase">
            {{ dropDatabaseDeleting ? 'Dropping…' : 'Drop' }}
          </button>
        </div>
      </div>
    </div>

    <!-- Drop Collection confirm -->
    <div v-if="dropCollectionTarget" class="del-overlay" @mousedown.self="dropCollectionTarget = null">
      <div class="del-dialog">
        <div class="del-title">
          <div class="t">Drop Collection</div>
          <button class="close-btn" @click="dropCollectionTarget = null">
            <BaseIcon name="close" :size="14" />
          </button>
        </div>
        <div class="del-body">
          <p>Are you sure you want to drop "<strong>{{ dropCollectionTarget.collName }}</strong>"? This deletes all of its documents and cannot be undone.</p>
          <div v-if="dropCollectionError" class="del-error">{{ dropCollectionError }}</div>
        </div>
        <div class="del-footer">
          <span class="spacer"></span>
          <button class="btn" @click="dropCollectionTarget = null">Cancel</button>
          <button class="btn danger" :disabled="dropCollectionDeleting" @click="confirmDropCollection">
            {{ dropCollectionDeleting ? 'Dropping…' : 'Drop' }}
          </button>
        </div>
      </div>
    </div>

    <!-- Rename Collection modal -->
    <div v-if="renameCollectionTarget" class="del-overlay" @mousedown.self="renameCollectionTarget = null">
      <div class="del-dialog">
        <div class="del-title">
          <div class="t">Rename Collection</div>
          <button class="close-btn" @click="renameCollectionTarget = null">
            <BaseIcon name="close" :size="14" />
          </button>
        </div>
        <div class="del-body">
          <input
            v-model="renameCollectionName"
            class="prompt-input"
            placeholder="New collection name"
            spellcheck="false"
            autocorrect="off"
            autocapitalize="off"
            @keydown.enter="confirmRenameCollection"
          />
          <div v-if="renameCollectionError" class="del-error">{{ renameCollectionError }}</div>
        </div>
        <div class="del-footer">
          <span class="spacer"></span>
          <button class="btn" @click="renameCollectionTarget = null">Cancel</button>
          <button class="btn primary" :disabled="!renameCollectionName.trim() || renameCollectionName.trim() === renameCollectionTarget.collName || renameCollectionSaving" @click="confirmRenameCollection">
            {{ renameCollectionSaving ? 'Renaming…' : 'Rename' }}
          </button>
        </div>
      </div>
    </div>

    <!-- Duplicate Collection prompt -->
    <div v-if="duplicateCollectionTarget" class="del-overlay" @mousedown.self="duplicateCollectionTarget = null">
      <div class="del-dialog">
        <div class="del-title">
          <div class="t">Duplicate Collection</div>
          <button class="close-btn" @click="duplicateCollectionTarget = null">
            <BaseIcon name="close" :size="14" />
          </button>
        </div>
        <div class="del-body">
          <input
            v-model="duplicateCollectionName"
            class="prompt-input"
            placeholder="New collection name"
            spellcheck="false"
            autocorrect="off"
            autocapitalize="off"
            @keydown.enter="confirmDuplicateCollection"
          />
          <div v-if="duplicateCollectionError" class="del-error">{{ duplicateCollectionError }}</div>
        </div>
        <div class="del-footer">
          <span class="spacer"></span>
          <button class="btn" @click="duplicateCollectionTarget = null">Cancel</button>
          <button class="btn primary" :disabled="!duplicateCollectionName.trim() || duplicateCollectionName.trim() === duplicateCollectionTarget.collName || duplicateCollectionSaving" @click="confirmDuplicateCollection">
            {{ duplicateCollectionSaving ? 'Duplicating…' : 'Duplicate' }}
          </button>
        </div>
      </div>
    </div>

    <!-- Add Database modal -->
    <div v-if="addDatabaseTarget" class="del-overlay" @mousedown.self="addDatabaseTarget = null">
      <div class="del-dialog">
        <div class="del-title">
          <div class="t">Add Database</div>
          <button class="close-btn" @click="addDatabaseTarget = null">
            <BaseIcon name="close" :size="14" />
          </button>
        </div>
        <div class="del-body">
          <input
            v-model="newDatabaseName"
            class="prompt-input"
            placeholder="Database name"
            spellcheck="false"
            autocorrect="off"
            autocapitalize="off"
          />
          <input
            v-model="newDatabaseCollName"
            class="prompt-input"
            style="margin-top:8px"
            placeholder="First collection name"
            spellcheck="false"
            autocorrect="off"
            autocapitalize="off"
            @keydown.enter="confirmAddDatabase"
          />
          <p style="margin-top:8px;color:var(--text-faint);font-size:12px">MongoDB only creates a database once it holds a collection, so a first collection is required.</p>
          <div v-if="addDatabaseError" class="del-error">{{ addDatabaseError }}</div>
        </div>
        <div class="del-footer">
          <span class="spacer"></span>
          <button class="btn" @click="addDatabaseTarget = null">Cancel</button>
          <button class="btn primary" :disabled="!newDatabaseName.trim() || !newDatabaseCollName.trim() || addDatabaseSaving" @click="confirmAddDatabase">
            {{ addDatabaseSaving ? 'Creating…' : 'Create' }}
          </button>
        </div>
      </div>
    </div>

    <!-- Indexes modal -->
    <div v-if="indexesTarget" class="del-overlay" @mousedown.self="closeIndexesModal()">
      <div class="del-dialog idx-dialog">
        <div class="del-title">
          <div class="t">Indexes — {{ indexesTarget.collName }}</div>
          <button class="close-btn" @click="closeIndexesModal()">
            <BaseIcon name="close" :size="14" />
          </button>
        </div>
        <div class="del-body">
          <div v-if="indexesLoading" class="idx-msg">Loading indexes…</div>
          <table v-else-if="indexesList.length" class="idx-table">
            <thead>
              <tr><th>Name</th><th>Keys</th><th>Unique</th><th>Hidden</th><th></th></tr>
            </thead>
            <tbody>
              <tr
                v-for="idx in indexesList"
                :key="idx.name"
                class="idx-row"
                :class="{ selected: selectedIndex && selectedIndex.name === idx.name }"
                @click="selectedIndex = idx"
              >
                <td class="idx-name">{{ idx.name }}</td>
                <td class="idx-keys">{{ indexKeyLabel(idx) }}</td>
                <td>{{ idx.unique ? 'Yes' : '—' }}</td>
                <td>{{ isIndexHidden(idx) ? 'Yes' : '—' }}</td>
                <td class="idx-actions">
                  <button
                    v-if="idx.name !== '_id_'"
                    class="btn"
                    :class="{ danger: pendingDropIndex === idx.name }"
                    @click.stop="dropIndex(idx.name)"
                  >{{ pendingDropIndex === idx.name ? 'Confirm' : 'Drop' }}</button>
                </td>
              </tr>
            </tbody>
          </table>
          <div v-else class="idx-msg">No indexes.</div>
          <div class="idx-hint">Select an index row to enable the Index menu.</div>

          <div class="idx-create">
            <div class="idx-create-title">{{ indexFormMode === 'edit' ? 'Edit index' : 'Create index' }}</div>
            <input
              v-model="newIndexKeys"
              class="prompt-input"
              placeholder='Keys, e.g. {"field": 1}'
              spellcheck="false"
              autocorrect="off"
              autocapitalize="off"
            />
            <input
              v-model="newIndexName"
              class="prompt-input"
              style="margin-top:8px"
              placeholder="Index name (optional)"
              spellcheck="false"
              autocorrect="off"
              autocapitalize="off"
            />
            <label class="idx-unique">
              <input type="checkbox" v-model="newIndexUnique" />
              <span>Unique</span>
            </label>
            <button v-if="indexFormMode === 'edit'" class="btn idx-cancel-edit" @click="resetIndexForm()">
              Cancel edit
            </button>
          </div>

          <div v-if="indexesError" class="del-error">{{ indexesError }}</div>
        </div>
        <div class="del-footer">
          <span class="spacer"></span>
          <button class="btn" @click="closeIndexesModal()">Close</button>
          <button class="btn primary" :disabled="!newIndexKeys.trim() || indexCreating" @click="confirmCreateIndex">
            {{ indexCreating ? (indexFormMode === 'edit' ? 'Saving…' : 'Creating…') : (indexFormMode === 'edit' ? 'Save changes' : 'Create index') }}
          </button>
        </div>
      </div>
    </div>

    <!-- Index: View Details (read-only) -->
    <div v-if="indexDetailsTarget" class="del-overlay" @mousedown.self="indexDetailsTarget = null">
      <div class="del-dialog idx-dialog">
        <div class="del-title">
          <div class="t">Index Details — {{ indexDetailsTarget.name }}</div>
          <button class="close-btn" @click="indexDetailsTarget = null">
            <BaseIcon name="close" :size="14" />
          </button>
        </div>
        <div class="del-body">
          <div class="idx-detail-section">Definition</div>
          <pre class="idx-detail-json">{{ indexSpecJson(indexDetailsTarget) }}</pre>
          <div class="idx-detail-section">Usage</div>
          <div v-if="indexDetailsLoading" class="idx-msg">Loading usage…</div>
          <table v-else-if="indexDetailsStats" class="idx-detail-stats">
            <tbody>
              <tr><td>Operations</td><td>{{ indexDetailsStats.accesses?.ops ?? '—' }}</td></tr>
              <tr><td>Tracking since</td><td>{{ formatIndexSince(indexDetailsStats.accesses?.since) }}</td></tr>
            </tbody>
          </table>
          <div v-else class="idx-msg">Usage statistics unavailable.</div>
        </div>
        <div class="del-footer">
          <span class="spacer"></span>
          <button class="btn" @click="indexDetailsTarget = null">Close</button>
        </div>
      </div>
    </div>

    <!-- Index: Drop confirmation (type the name to confirm) -->
    <div v-if="dropIndexTarget" class="del-overlay" @mousedown.self="dropIndexTarget = null">
      <div class="del-dialog">
        <div class="del-title">
          <div class="t">Drop Index</div>
          <button class="close-btn" @click="dropIndexTarget = null">
            <BaseIcon name="close" :size="14" />
          </button>
        </div>
        <div class="del-body">
          <p>This permanently drops the index
            <code>{{ dropIndexTarget.name }}</code>. Queries that relied on it may slow down.
            This cannot be undone.</p>
          <p class="cc-prompt">Type <code>{{ dropIndexTarget.name }}</code> to confirm:</p>
          <input
            class="prompt-input"
            v-model="dropIndexConfirmText"
            spellcheck="false"
            autocomplete="off"
            autocorrect="off"
            autocapitalize="off"
            @keydown.enter="confirmDropIndex"
          />
          <div v-if="dropIndexError" class="del-error">{{ dropIndexError }}</div>
        </div>
        <div class="del-footer">
          <span class="spacer"></span>
          <button class="btn" @click="dropIndexTarget = null">Cancel</button>
          <button
            class="btn danger"
            :disabled="dropIndexBusy || dropIndexConfirmText !== dropIndexTarget.name"
            @click="confirmDropIndex"
          >{{ dropIndexBusy ? 'Dropping…' : 'Drop Index' }}</button>
        </div>
      </div>
    </div>
</template>

<!-- Same stylesheet App.vue uses; scoped here so the dialog classes (.del-*, .idx-*,
     .btn, …) apply to these modals without leaking globally to other components. -->
<style src="../../App.css" scoped></style>
