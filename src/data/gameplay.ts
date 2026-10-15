// Difficulty & gameplay settings of enshrouded_server.json (`gameSettings`), taken from the
// readme the dedicated server ships (version 0.9.0.0) – names, ranges and defaults as Keen
// documents them. The server only applies them when `gameSettingsPreset` is "Custom".

export type SettingKind = 'factor' | 'bool' | 'enum' | 'minutes';

export interface GameSetting {
  key: string;
  label: string;
  description: string;
  kind: SettingKind;
  default: number | boolean | string;
  min?: number;
  max?: number;
  step?: number;
  options?: { value: string; label: string }[];
  group: GroupId;
}

export type GroupId = 'player' | 'survival' | 'world' | 'progress' | 'enemies' | 'bosses';

export const GROUPS: { id: GroupId; label: string; blurb: string }[] = [
  { id: 'player', label: 'Player', blurb: 'Health, mana, stamina and how long you last in the cold and under water.' },
  { id: 'survival', label: 'Survival & death', blurb: 'Hunger, durability, the Shroud and what you lose when you die.' },
  { id: 'world', label: 'World & time', blurb: 'Day and night length, weather, gliding, fishing and taming.' },
  { id: 'progress', label: 'Resources & progression', blurb: 'Mining, farming, crafting speed, loot and experience.' },
  { id: 'enemies', label: 'Enemies', blurb: 'How many enemies there are, how hard they hit and how far they notice you.' },
  { id: 'bosses', label: 'Bosses', blurb: 'Boss damage and health.' },
];

const opt = (...v: [string, string][]) => v.map(([value, label]) => ({ value, label }));

export const GAME_SETTINGS: GameSetting[] = [
  { key: 'playerHealthFactor', label: 'Player health', description: 'Scales players’ maximum health.', kind: 'factor', default: 1, min: 0.25, max: 4, group: 'player' },
  { key: 'playerManaFactor', label: 'Player mana', description: 'Scales players’ maximum mana.', kind: 'factor', default: 1, min: 0.25, max: 4, group: 'player' },
  { key: 'playerStaminaFactor', label: 'Player stamina', description: 'Scales players’ maximum stamina.', kind: 'factor', default: 1, min: 0.25, max: 4, group: 'player' },
  { key: 'playerBodyHeatFactor', label: 'Body heat', description: 'How long players can stay in very cold areas before hypothermia sets in.', kind: 'factor', default: 1, min: 0.5, max: 2, group: 'player' },
  { key: 'playerDivingTimeFactor', label: 'Diving time', description: 'Initial oxygen – how long players can stay under water.', kind: 'factor', default: 1, min: 0.5, max: 2, group: 'player' },

  { key: 'enableDurability', label: 'Weapon durability', description: 'Weapons wear out and break. Off = weapons never break.', kind: 'bool', default: true, group: 'survival' },
  { key: 'enableStarvingDebuff', label: 'Hunger & starvation', description: 'Starving players lose health over time until they eat or drink.', kind: 'bool', default: false, group: 'survival' },
  { key: 'foodBuffDurationFactor', label: 'Food buff duration', description: 'How long food buffs last.', kind: 'factor', default: 1, min: 0.5, max: 2, group: 'survival' },
  { key: 'fromHungerToStarving', label: 'Hungry before starving', description: 'How long the “hungry” state lasts before starving sets in.', kind: 'minutes', default: 600_000_000_000, min: 5, max: 20, step: 1, group: 'survival' },
  { key: 'shroudTimeFactor', label: 'Time in the Shroud', description: 'How long characters can stay inside the Shroud.', kind: 'factor', default: 1, min: 0.5, max: 2, group: 'survival' },
  {
    key: 'tombstoneMode',
    label: 'On death',
    description: 'What players drop into their tombstone when they die.',
    kind: 'enum',
    default: 'AddBackpackMaterials',
    options: opt(['AddBackpackMaterials', 'Drop materials'], ['Everything', 'Drop everything'], ['NoTombstone', 'Keep everything']),
    group: 'survival',
  },
  {
    key: 'curseModifier',
    label: 'Shroud curse',
    description: 'Chance of receiving the Shroud curse from cursed enemies. Easy turns the curse off, Hard doubles it.',
    kind: 'enum',
    default: 'Normal',
    options: opt(['Easy', 'Off'], ['Normal', 'Normal'], ['Hard', 'Double']),
    group: 'survival',
  },

  { key: 'dayTimeDuration', label: 'Day length', description: 'Real minutes of daylight.', kind: 'minutes', default: 1_800_000_000_000, min: 2, max: 60, step: 1, group: 'world' },
  { key: 'nightTimeDuration', label: 'Night length', description: 'Real minutes of night.', kind: 'minutes', default: 720_000_000_000, min: 2, max: 60, step: 1, group: 'world' },
  {
    key: 'weatherFrequency',
    label: 'Weather',
    description: 'How often new weather phenomena appear.',
    kind: 'enum',
    default: 'Normal',
    options: opt(['Disabled', 'Off'], ['Rare', 'Rare'], ['Normal', 'Normal'], ['Often', 'Often']),
    group: 'world',
  },
  { key: 'enableGliderTurbulences', label: 'Glider turbulence', description: 'Air turbulence affects the glider. Off = gliding works like in early versions.', kind: 'bool', default: true, group: 'world' },
  {
    key: 'fishingDifficulty',
    label: 'Fishing difficulty',
    description: 'How strong fish are in the fishing minigame.',
    kind: 'enum',
    default: 'Normal',
    options: opt(['VeryEasy', 'Very easy'], ['Easy', 'Easy'], ['Normal', 'Normal'], ['Hard', 'Hard'], ['VeryHard', 'Very hard']),
    group: 'world',
  },
  {
    key: 'tamingStartleRepercussion',
    label: 'Startled while taming',
    description: 'What happens to taming progress (the hearts) when wildlife gets startled.',
    kind: 'enum',
    default: 'LoseSomeProgress',
    options: opt(['KeepProgress', 'Keep progress'], ['LoseSomeProgress', 'Lose some'], ['LoseAllProgress', 'Lose all']),
    group: 'world',
  },

  { key: 'miningDamageFactor', label: 'Mining effectiveness', description: 'More terraforming and more resources per hit.', kind: 'factor', default: 1, min: 0.5, max: 2, group: 'progress' },
  { key: 'plantGrowthSpeedFactor', label: 'Plant growth speed', description: 'How fast crops grow.', kind: 'factor', default: 1, min: 0.25, max: 2, group: 'progress' },
  { key: 'resourceDropStackAmountFactor', label: 'Resource yield', description: 'Materials per loot stack from chests, enemies etc.', kind: 'factor', default: 1, min: 0.25, max: 2, group: 'progress' },
  { key: 'factoryProductionSpeedFactor', label: 'Workstation speed', description: 'Production speed of workshop items.', kind: 'factor', default: 1, min: 0.25, max: 2, group: 'progress' },
  { key: 'perkUpgradeRecyclingFactor', label: 'Rune refund on salvage', description: 'Share of runes returned when salvaging upgraded weapons.', kind: 'factor', default: 0.5, min: 0, max: 1, group: 'progress' },
  { key: 'perkCostFactor', label: 'Weapon upgrade cost', description: 'Runes needed to upgrade weapons.', kind: 'factor', default: 1, min: 0.25, max: 2, group: 'progress' },
  { key: 'experienceCombatFactor', label: 'Combat XP', description: 'Experience from fighting.', kind: 'factor', default: 1, min: 0.25, max: 2, group: 'progress' },
  { key: 'experienceMiningFactor', label: 'Mining XP', description: 'Experience from mining resources.', kind: 'factor', default: 1, min: 0, max: 2, group: 'progress' },
  { key: 'experienceExplorationQuestsFactor', label: 'Exploration & quest XP', description: 'Experience from exploring and completing quests.', kind: 'factor', default: 1, min: 0.25, max: 2, group: 'progress' },

  {
    key: 'randomSpawnerAmount',
    label: 'Enemy amount',
    description: 'How many enemies roam the world.',
    kind: 'enum',
    default: 'Normal',
    options: opt(['Few', 'Few'], ['Normal', 'Normal'], ['Many', 'Many'], ['Extreme', 'Extreme']),
    group: 'enemies',
  },
  {
    key: 'aggroPoolAmount',
    label: 'Simultaneous attackers',
    description: 'How many enemies may attack at the same time.',
    kind: 'enum',
    default: 'Normal',
    options: opt(['Few', 'Few'], ['Normal', 'Normal'], ['Many', 'Many'], ['Extreme', 'Extreme']),
    group: 'enemies',
  },
  { key: 'enemyDamageFactor', label: 'Enemy damage', description: 'Damage of all enemies except bosses.', kind: 'factor', default: 1, min: 0.25, max: 5, group: 'enemies' },
  { key: 'enemyHealthFactor', label: 'Enemy health', description: 'Health of all enemies except bosses.', kind: 'factor', default: 1, min: 0.25, max: 4, group: 'enemies' },
  { key: 'enemyStaminaFactor', label: 'Enemy stun resistance', description: 'Enemy stamina – higher takes longer to stun (not bosses).', kind: 'factor', default: 1, min: 0.5, max: 2, group: 'enemies' },
  { key: 'enemyPerceptionRangeFactor', label: 'Enemy perception', description: 'How far enemies see and hear players (not bosses).', kind: 'factor', default: 1, min: 0.5, max: 2, group: 'enemies' },
  { key: 'threatBonus', label: 'Enemy attack frequency', description: 'How often enemies attack (not bosses).', kind: 'factor', default: 1, min: 0.25, max: 4, group: 'enemies' },
  { key: 'pacifyAllEnemies', label: 'Pacified enemies', description: 'Enemies only fight back once attacked (bosses still fight).', kind: 'bool', default: false, group: 'enemies' },

  { key: 'bossDamageFactor', label: 'Boss damage', description: 'Damage of boss attacks.', kind: 'factor', default: 1, min: 0.2, max: 5, group: 'bosses' },
  { key: 'bossHealthFactor', label: 'Boss health', description: 'Health of bosses.', kind: 'factor', default: 1, min: 0.2, max: 5, group: 'bosses' },
];

export const DEFAULT_GAME_SETTINGS: Record<string, number | string | boolean> = Object.fromEntries(GAME_SETTINGS.map((s) => [s.key, s.default]));

export const PRESETS: { id: 'Default' | 'Relaxed' | 'Hard' | 'Survival' | 'Custom'; label: string; blurb: string }[] = [
  { id: 'Default', label: 'Default', blurb: 'The game as designed – recommended for a first playthrough.' },
  { id: 'Relaxed', label: 'Relaxed', blurb: 'Fewer enemies, more resources and loot. For builders and cosy adventures.' },
  { id: 'Hard', label: 'Hard', blurb: 'More enemies, and more aggressive ones – a tougher fight.' },
  { id: 'Survival', label: 'Survival', blurb: 'Hard, plus survival mechanics such as hunger. For those who seek punishment.' },
  { id: 'Custom', label: 'Custom', blurb: 'Tune every value below yourself.' },
];

/** Server-browser filter tags (readme, 0.9). */
export const TAGS: { group: string; tags: { id: string; label: string }[] }[] = [
  { group: 'Players', tags: [{ id: 'LookingForPlayers', label: 'Looking for players' }] },
  {
    group: 'Focus',
    tags: [
      { id: 'BaseBuilding', label: 'Base building' },
      { id: 'Exploration', label: 'Exploration' },
      { id: 'Roleplay', label: 'Roleplay' },
    ],
  },
  {
    group: 'Language',
    tags: ['English', 'German', 'French', 'Italian', 'Japanese', 'Korean', 'Polish', 'Portuguese', 'Russian', 'Spanish', 'Thai', 'Turkish', 'Ukrainian', 'Chinese', 'Taiwanese'].map((l) => ({ id: l, label: l })),
  },
];

/** Value of a setting in a config, falling back to the documented default. */
export function settingValue(gs: Record<string, number | string | boolean>, s: GameSetting) {
  return gs[s.key] ?? s.default;
}

export function isDefault(gs: Record<string, number | string | boolean>, s: GameSetting) {
  const v = settingValue(gs, s);
  return typeof v === 'number' && typeof s.default === 'number' ? Math.abs(v - s.default) < 1e-6 : v === s.default;
}

export const PRIORITIES = [
  { value: 'idle', label: 'Low' },
  { value: 'belowNormal', label: 'Below normal' },
  { value: 'normal', label: 'Normal' },
  { value: 'aboveNormal', label: 'Above normal' },
  { value: 'high', label: 'High' },
];

/** Starter roles – the four a fresh server generates, plus handy extras. */
export const ROLE_TEMPLATES = [
  { name: 'Admin', blurb: 'Everything, including kick & ban', canKickBan: true, canAccessInventories: true, canEditWorld: true, canEditBase: true, canExtendBase: true },
  { name: 'Friend', blurb: 'Build, use chests – no altars or bans', canKickBan: false, canAccessInventories: true, canEditWorld: true, canEditBase: true, canExtendBase: false },
  { name: 'Guest', blurb: 'Play and dig, but hands off bases and chests', canKickBan: false, canAccessInventories: false, canEditWorld: true, canEditBase: false, canExtendBase: false },
  { name: 'Visitor', blurb: 'Look around – can’t change anything', canKickBan: false, canAccessInventories: false, canEditWorld: false, canEditBase: false, canExtendBase: false },
  { name: 'Builder', blurb: 'Bases and altars, but no chests', canKickBan: false, canAccessInventories: false, canEditWorld: true, canEditBase: true, canExtendBase: true },
  { name: 'Moderator', blurb: 'Kick & ban without base rights', canKickBan: true, canAccessInventories: false, canEditWorld: true, canEditBase: false, canExtendBase: false },
];

export const PERMISSIONS = [
  { key: 'canKickBan', label: 'Kick & ban', description: 'Kick or permanently ban other players.' },
  { key: 'canAccessInventories', label: 'Chests & stations', description: 'Use chests, workstations and other containers in bases.' },
  { key: 'canEditWorld', label: 'Change the world', description: 'Terraform and destroy outside bases. Off for anti-griefing visitors.' },
  { key: 'canEditBase', label: 'Build in bases', description: 'Build, terraform and remove constructions inside bases (needs “Change the world”).' },
  { key: 'canExtendBase', label: 'Flame Altars', description: 'Place, remove and upgrade Flame Altars.' },
] as const;
