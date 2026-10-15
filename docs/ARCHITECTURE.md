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
│  commands/export_commands.rs                │
│  commands/app_commands.rs                   │
│  commands/tab_commands.rs                   │
│  commands/window_commands.rs                │
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
│   ├── UndoNotice.svelte
│   └── UpdateBanner.svelte
├── domain/                 # Чистые функции (зеркало Rust)
│   ├── taskRules.ts
│   └── taskValidation.ts
├── i18n/                   # Словари ru/en и переключение языка
│   ├── en.ts
│   ├── index.svelte.ts
│   └── ru.ts
├── lib/                    # Чистые функции и типы
│   ├── capture.ts          # ритм полей ввода задач
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
│   ├── updateService.ts
│   └── windowService.ts
└── stores/
    ├── appStore.svelte.ts
    ├── dbStore.svelte.ts
    ├── dragStore.svelte.ts      # pointer-перетаскивание
    ├── settingsStore.svelte.ts
    ├── tabStore.svelte.ts
    ├── taskStore.svelte.ts
    ├── uiStore.svelte.ts
    └── updateStore.svelte.ts
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

Все обращения к данным идут через сервисы в `src/services/` — по одному на
область (`taskService`, `tabService`, `settingsService`, `exportService`,
`appService`, `dbService`, `updateService`, `windowService`). Все они, кроме
`updateService`, вызывают Tauri commands через `invoke`; проверка обновлений
идёт через JS API плагина updater.

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
│   ├── app_commands.rs
│   ├── export_commands.rs
│   ├── mod.rs
│   ├── settings_commands.rs
│   ├── tab_commands.rs
│   ├── task_commands.rs
│   └── window_commands.rs
├── db/                     # Слой данных
│   ├── mod.rs              # Инициализация, Db State
│   ├── migrations.rs       # Миграции схемы
│   └── repository.rs       # CRUD-операции
├── domain/                 # Доменная логика
│   ├── mod.rs
│   ├── task.rs             # Типы задач, TaskNode, SearchResult
│   ├── rules.rs            # Правила завершения (compute_toggle_plan)
│   ├── transfer.rs         # Формат экспорта/импорта (JSON)
│   └── validation.rs       # Валидация дат и названий
├── error.rs                # Типизированные ошибки
└── platform/               # Платформенный слой
    ├── mod.rs
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
- Текущая версия: 5 — `create_tasks_and_settings` (1), `add_tabs` (2), `add_task_comment` (3), `add_task_search_text` (4), `add_tab_deleted_at` (5)

**Транзакции:**
- Все операции записи выполняются в транзакциях
- Каскадные операции (завершение родителя, удаление) атомарны

**Параметризованные запросы:**
- Все SQL-запросы используют параметры (`?1`, `?2`, ...)
- Защита от SQL-инъекций

### Платформенный слой

**Трей (`platform/tray.rs`):**
- Иконка в системном трее / меню-баре
- Левый клик — показать окно (скрытия по клику нет)
- Меню: «Open QuiTa / Открыть QuiTa», «About / О программе», разделитель, «Quit / Выйти»
- Подписи двуязычные: меню строит система до того, как известен язык интерфейса

Цвет значка выбирается по теме, а не зашит в картинку. Форма строки задана
альфа-каналом в `icons/tray-icon.svg`, а пиксели перекрашиваются при загрузке:
тёмная панель задач требует светлого значка, иначе он в ней пропадает. Значение
берётся из `window.theme()`; если тему прочитать не удалось, берётся светлый
значок — так ждут Windows и обычные панели Linux. На macOS цвет всё равно
игнорируется: `icon_as_template(true)` перерисовывает значок под цвет строки
меню.

**Горячие клавиши (`platform/shortcuts.rs`):**

| Платформа | Сочетание | Почему не другое |
|-----------|-----------|------------------|
| macOS | `CmdOrCtrl+Shift+Q` | Соглашение системы для утилит, вызываемых откуда угодно |
| Windows | `Super+Alt+Q` | `Ctrl+Shift+Q` не зарезервирован системой, но его часто занимают чужие глобальные хоткеи — Telegram, JetBrains, VPN-клиенты; проиграв гонку, сочетание не сработает |
| Linux | `Ctrl+Alt+Q` | `Super` перехватывают оконные менеджеры: «показать рабочий стол», смена пространств |

- Подпись в настройках берётся из того же места, что и регистрация, — разойтись
  они не могут
- При нажатии окно показывается и фокусируется
- Событие `quita:global-shortcut` отправляется в UI
- Ошибка регистрации не фатальна — приложение работает через трей

Оговорка для Linux: на раскладках, где `Ctrl+Alt` даёт AltGr (немецкая,
польская и другие), сочетание может не дойти как `Ctrl+Alt+Q`. Это свойство
раскладки, а не приложения — поэтому сочетание названо явно, а не зашито.

## Почему rusqlite bundled

- **Офлайн** — не требуется внешний SQLite
- **Простота сборки** — нет зависимостей от системных библиотек
- **Предсказуемость** — одинаковая версия SQLite на всех платформах

## Безопасность

### Capabilities (src-tauri/capabilities/default.json)

Минимальный набор разрешений — каждый пункт обоснован:

| Разрешение | Зачем |
|------------|-------|
| `core:default` | Базовые разрешения Tauri |
| `dialog:allow-save` | Сохранение файла (экспорт) |
| `dialog:allow-open` | Открытие файла (импорт) |
| `updater:default` | Проверка и установка обновления |
| `process:allow-restart` | Перезапуск после установленного обновления |
| `opener:allow-open-url` | Ссылка на репозиторий в «О программе» |

Плагины файловой системы, shell, HTTP и process-команд (кроме перезапуска)
не выданы.

**Открытие ссылки ограничено одним репозиторием**, а не всеми `https`-адресами:

```json
{
  "identifier": "opener:allow-open-url",
  "allow": [{ "url": "https://github.com/zeroscrypt/QuiTa*" }]
}
```

Это осознанно уже `opener:default`, который разрешил бы открыть любой
`http(s)`-адрес: при инъекции скрипта в вебвью этого достаточно, чтобы
приложение открыло произвольную страницу от имени пользователя.

Горячая клавиша регистрируется из Rust (`tauri-plugin-global-shortcut`), поэтому
webview не нуждается в `global-shortcut:*`.

### Content Security Policy

CSP задан в `tauri.conf.json` и ограничивает загрузку кода локальным:

```
default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline';
img-src 'self' data: asset: http://asset.localhost;
font-src 'self' data:; connect-src 'self' ipc: http://ipc.localhost
```

`script-src 'self'` запрещает и внешние скрипты, и `unsafe-eval`. В разметке нет
`{@html}` и `innerHTML`, поэтому пользовательский текст всегда экранируется Svelte.

Сеть в `connect-src` не добавлена намеренно: плагин обновления обращается к
GitHub из Rust, а не из вебвью, поэтому CSP остаётся замкнутым на `self` и IPC.

### Граница доверия при импорте

Файл приходит откуда угодно, поэтому он недоверенный ввод:

| Что | Защита |
|-----|--------|
| Размер файла | Отказ свыше 64 МБ до чтения — иначе пользователь, по ошибке указавший много гигабайт, вытянет файл в память целиком |
| Формат | Только JSON; YAML и XML отклоняются с объяснением |
| Заголовок задачи | Проверяется на те же 500 символов, что и ввод с клавиатуры; слишком длинная строка отбрасывается и попадает в счётчик `skipped` |
| Цвет вкладки | Принимается только из палитры |
| Идентификаторы | Слияние по `id`, ничего не удаляется |

Весь SQL параметризован: подстановка в строку запроса используется только для
константы `TASK_COLUMNS`, объявленной в одном месте.

## Структура репозитория

```
QuiTa/
├── .github/
│   └── workflows/
  │       ├── ci.yml              # Линт, тесты, сборка, аудит зависимостей
  │       ├── build.yml           # Сборка установщиков под тег
  │       └── release.yml         # Версия, changelog, теги, автослияние
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
│   │   ├── integration_tests.rs
│   │   └── documentation.rs
│   ├── Cargo.toml
│   └── tauri.conf.json
├── tests/                      # Тесты фронтенда (vitest)
├── package.json
├── pnpm-lock.yaml
├── tsconfig.json
├── vite.config.ts
└── README.md
```
