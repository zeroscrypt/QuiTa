# Архитектура QuiTa

## Обзор

QuiTa — desktop-приложение на Tauri 2, использующее Svelte 5 для фронтенда и Rust для бэкенда. Все данные хранятся локально в SQLite.

## Слои приложения

```
┌─────────────────────────────────────────────┐
│                  UI (Svelte)                │
│  App.svelte, components/, stores/           │
└──────────────────┬──────────────────────────┘
                   │ invoke / listen
┌──────────────────▼──────────────────────────┐
│              Tauri Commands                 │
│  commands/task_commands.rs                  │
│  commands/settings_commands.rs              │
│  commands/export_commands.rs              │
└──────────────────┬──────────────────────────┘
                   │
┌──────────────────▼──────────────────────────┐
│              Repository                      │
│  db/repository.rs                           │
└──────────────────┬──────────────────────────┘
                   │
┌──────────────────▼──────────────────────────┐
│              SQLite (rusqlite)               │
│  db/migrations.rs                           │
└─────────────────────────────────────────────┘
```

## Фронтенд (Svelte 5 + TypeScript)

### Структура

```
src/
├── App.svelte              # Корневой компонент
├── components/             # UI-компоненты
│   ├── AboutDialog.svelte
│   ├── AddTaskInput.svelte
│   ├── AppHeader.svelte
│   ├── AutoDeleteDaysPopover.svelte
│   ├── CommentPopover.svelte
│   ├── ConfirmDialog.svelte
│   ├── ContextMenu.svelte
│   ├── DragLayer.svelte
│   ├── DueDatePicker.svelte
│   ├── EmptyState.svelte
│   ├── IconCheckbox.svelte
│   ├── InfoHint.svelte
│   ├── InlineEdit.svelte
│   ├── SearchBar.svelte
│   ├── SettingsPanel.svelte
│   ├── SubtaskList.svelte
│   ├── TabsBar.svelte
│   ├── TaskItem.svelte
│   ├── TaskList.svelte
│   └── UndoNotice.svelte
├── domain/                 # Чистые функции (зеркало Rust)
│   ├── taskRules.ts
│   └── taskValidation.ts
├── lib/                    # Чистые функции и типы
│   ├── menuTypes.ts
│   ├── scroll.ts           # прокрутка по прямоугольникам
│   ├── text.ts             # обрезка по длине, не по словам
│   ├── trash.ts            # иерархия задач в корзине
│   └── types.ts            # Типы данных (зеркало Rust)
├── services/               # Обёртки над Tauri commands
│   ├── appService.ts
│   ├── dbService.ts
│   ├── exportService.ts
│   ├── settingsService.ts
│   ├── tabService.ts
│   ├── taskService.ts
│   └── windowService.ts
└── stores/
    ├── dbStore.svelte.ts
    ├── dragStore.svelte.ts      # pointer-перетаскивание
    ├── settingsStore.svelte.ts
    ├── tabStore.svelte.ts
    ├── taskStore.svelte.ts
    └── uiStore.svelte.ts
```

> **Важно:** модули с runes обязаны иметь расширение `.svelte.ts`.
> Плагин Svelte не преобразует runes в обычных `.ts` файлах — такой модуль
> упадёт в рантайме с `ReferenceError` (окно останется пустым), при этом
> `svelte-check` и `vitest` ошибку не покажут. В `vite.config.ts` для этого
> добавлен `resolve.extensions` с `.svelte.ts` первым.

### Управление состоянием

- **Svelte 5 runes** — `$state`, `$derived`, `$effect` (в компонентах и в `*.svelte.ts` модулях)
- **Stores** — классы с реактивными полями в `stores/*.svelte.ts`
- **Tauri events** — `listen` для событий от Rust (например, `quita:global-shortcut`)

### Связь с бэкендом

Все обращения к данным через `src/services/taskService.ts`, который вызывает Tauri commands через `invoke`.

### Контракт сериализации

Rust-структуры, которые уходят в UI, помечены `#[serde(rename_all = "camelCase")]`,
поэтому JSON совпадает с моделью `src/lib/types.ts` (`completedAt`, `parentId`,
`sortOrder`, `showCompleted`, `matchedSubtasks`).

Без этого атрибута TypeScript получает `undefined` вместо значения (например
`completed_at: null` → `task.completedAt === undefined`), и `task.completedAt !== null`
даёт `true` — то есть каждая задача выглядит завершённой. Контракт зафиксирован
тестами `task_serializes_to_camel_case` и `task_deserializes_from_camel_case`
в `src-tauri/src/domain/task.rs`.

## Бэкенд (Rust)

### Структура

```
src-tauri/src/
├── lib.rs                  # Точка входа Tauri
├── main.rs                 # Точка входа приложения
├── commands/               # Tauri commands
│   ├── export_commands.rs
│   ├── settings_commands.rs
│   └── task_commands.rs
├── db/                     # Слой данных
│   ├── mod.rs              # Инициализация, Db State
│   ├── migrations.rs       # Миграции схемы
│   └── repository.rs       # CRUD-операции
├── domain/                 # Доменная логика
│   ├── mod.rs
│   ├── task.rs             # Типы задач, TaskNode, SearchResult
│   ├── rules.rs            # Правила завершения (compute_toggle_plan)
│   └── validation.rs       # Валидация дат и названий
├── error.rs                # Типизированные ошибки
└── platform/               # Платформенный слой
    ├── tray.rs             # Трей-иконка
    └── shortcuts.rs        # Глобальные горячие клавиши
```

### Доменная логика

**Принцип:** вся бизнес-логика в чистых функциях в `domain/`, которые не зависят от Tauri и SQLite.

**`domain/rules.rs`:**
- `compute_toggle_plan` — чистая функция, вычисляющая план завершения/открытия задачи
- `TogglePlan` — структура с описанием изменений (какие задачи завершить, какие восстановить). Родитель в плане не участвует: подзадача никогда не меняет его состояние
- Функция используется как в repository (для применения), так и в тестах (для проверки)

**`domain/validation.rs`:**
- `normalize_title` — обрезка пробелов, отклонение пустых названий
- `is_valid_due_date` — строгая проверка формата YYYY-MM-DD через chrono

**`domain/task.rs`:**
- `Task` — основная сущность
- `TaskNode` — корневая задача с подзадачами (для UI)
- `SearchResult` — результаты поиска с контекстом родителя
- `AppSettings` — настройки (тема, показ завершённых)

### Слой данных

**Подключение:**
- `Db` — обёртка над `Mutex<Connection>`, управляется Tauri State
- Соединение создаётся при инициализации в `db::init`
- Режим WAL (`journal_mode = WAL`) для конкурентного доступа
- Включены внешние ключи (`foreign_keys = ON`)

**Миграции:**
- `db/migrations.rs` — таблица `schema_migrations` с версиями
- Миграции применяются последовательно при запуске
- Текущая версия: 1 (`create_tasks_and_settings`)

**Транзакции:**
- Все операции записи выполняются в транзакциях
- Каскадные операции (завершение родителя, удаление) атомарны

**Параметризованные запросы:**
- Все SQL-запросы используют параметры (`?1`, `?2`, ...)
- Защита от SQL-инъекций

### Платформенный слой

**Трей (`platform/tray.rs`):**
- Иконка в системном трее / меню-баре
- Левый клик — показать/скрыть окно
- Меню: «Открыть QuiTa» / «Выйти»

**Горячие клавиши (`platform/shortcuts.rs`):**
- `CmdOrCtrl+Shift+Q` — глобальная горячая клавиша
- При нажатии окно показывается и фокусируется
- Событие `quita:global-shortcut` отправляется в UI
- Ошибка регистрации не фатальна — приложение работает через трей

## Почему rusqlite bundled

- **Офлайн** — не требуется внешний SQLite
- **Простота сборки** — нет зависимостей от системных библиотек
- **Предсказуемость** — одинаковая версия SQLite на всех платформах

## Безопасность

### Capabilities (src-tauri/capabilities/default.json)

Минимальный набор разрешений:
- `core:default` — базовые разрешения Tauri
- `dialog:allow-save` — сохранение файлов (экспорт)
- `dialog:allow-open` — открытие файла (импорт)

Горячая клавиша регистрируется из Rust (`tauri-plugin-global-shortcut`), поэтому webview
не нуждается в `global-shortcut:*`. Плагины диалога, файловой системы, shell, HTTP и
process не выданы — даже при инъекции скрипта webview может вызвать только команды
приложения и открыть диалог сохранения.

### Content Security Policy

CSP задан в `tauri.conf.json` и ограничивает загрузку кода локальным:

```
default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline';
img-src 'self' data: asset: http://asset.localhost;
font-src 'self' data:; connect-src 'self' ipc: http://ipc.localhost
```

`script-src 'self'` запрещает и внешние скрипты, и `unsafe-eval`. В разметке нет
`{@html}` и `innerHTML`, поэтому пользовательский текст всегда экранируется Svelte.

## Структура репозитория

```
QuiTa/
├── .github/
│   └── workflows/
│       └── ci.yml              # CI: lint, тесты, сборка
├── docs/                       # Документация
├── src/                        # Фронтенд (Svelte)
│   ├── components/            # UI
│   ├── domain/                # правила, повторяемые на бэкенде
│   ├── lib/                   # чистые функции: scroll, text, trash, типы
│   ├── services/              # вызовы Tauri-команд
│   ├── stores/                # состояние (taskStore, dragStore, uiStore…)
│   ├── App.svelte
│   └── main.ts
├── src-tauri/                  # Бэкенд (Rust)
│   ├── capabilities/
│   ├── icons/
│   ├── src/
│   │   ├── commands/
│   │   ├── db/
│   │   ├── domain/
│   │   ├── platform/
│   │   ├── error.rs
│   │   ├── lib.rs
│   │   └── main.rs
│   ├── tests/
│   │   └── integration_tests.rs
│   ├── Cargo.toml
│   └── tauri.conf.json
├── tests/                      # Тесты фронтенда (vitest)
├── package.json
├── pnpm-lock.yaml
├── tsconfig.json
├── vite.config.ts
└── README.md
```
