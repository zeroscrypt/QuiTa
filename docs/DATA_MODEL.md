# Модель данных QuiTa

## Обзор

База данных SQLite с четырьмя таблицами: `tasks`, `tabs`, `settings`, `schema_migrations`. Все данные хранятся в файле `quita.db` в каталоге приложения Tauri.

## Диаграмма

```
┌─────────────────────────────────────────────┐
│                  tasks                      │
├─────────────────────────────────────────────┤
│ id (PK, TEXT)                               │
│ parent_id (FK → tasks.id, nullable)         │
│ tab_id (FK → tabs.id, nullable)             │
│ title (TEXT, NOT NULL)                      │
│ search_text (TEXT, NOT NULL)                │
│ created_at (TEXT, NOT NULL)                 │
│ updated_at (TEXT, NOT NULL)                 │
│ completed_at (TEXT, nullable)               │
│ deleted_at (TEXT, nullable)                 │
│ due_date (TEXT, nullable)                   │
│ comment (TEXT, nullable)                    │
│ sort_order (INTEGER, NOT NULL)              │
│ completion_event_id (TEXT, nullable)        │
│ auto_completed_by (TEXT, nullable)          │
└─────────────────────────────────────────────┘
      ▲ tab_id                    ↺ parent_id (один уровень)

┌─────────────────────────────────────────────┐
│                  tabs                       │
├─────────────────────────────────────────────┤
│ id (PK, TEXT)                               │
│ name (TEXT, NOT NULL)                       │
│ color (TEXT, NOT NULL)                      │
│ sort_order (INTEGER, NOT NULL)              │
│ created_at (TEXT, NOT NULL)                 │
│ updated_at (TEXT, NOT NULL)                 │
└─────────────────────────────────────────────┘

┌─────────────────────────────────────────────┐
│                 settings                    │
├─────────────────────────────────────────────┤
│ key (PK, TEXT)                              │
│ value (TEXT, NOT NULL)                      │
└─────────────────────────────────────────────┘

┌─────────────────────────────────────────────┐
│             schema_migrations               │
├─────────────────────────────────────────────┤
│ version (PK, INTEGER)                       │
│ name (TEXT, NOT NULL)                       │
│ applied_at (TEXT, NOT NULL)                 │
└─────────────────────────────────────────────┘
```

## Таблица `tasks`

### Поля

| Поле | Тип | NULL | Описание |
|------|-----|------|----------|
| `id` | TEXT | Нет | UUID v4, первичный ключ |
| `parent_id` | TEXT | Да | ID родительской задачи. NULL для корневых задач |
| `tab_id` | TEXT | Да | ID вкладки. NULL у строк, созданных до появления вкладок: они показываются в первой |
| `title` | TEXT | Нет | Название задачи (обрезанное) |
| `search_text` | TEXT | Нет | Название в нижнем регистре, пишется из Rust. Поиск идёт по нему: `LIKE` в SQLite складывает регистр только для ASCII, поэтому без этой копии «МОЛОКО» не нашло бы «Молоко» |
| `created_at` | TEXT | Нет | Время создания (RFC 3339 UTC) |
| `updated_at` | TEXT | Нет | Время последнего обновления (RFC 3339 UTC) |
| `completed_at` | TEXT | Да | Время завершения (RFC 3339 UTC). NULL если не завершена |
| `deleted_at` | TEXT | Да | Время мягкого удаления (RFC 3339 UTC). NULL если не удалена |
| `due_date` | TEXT | Да | Календарная дата (YYYY-MM-DD). NULL если не установлен |
| `comment` | TEXT | Да | Свободный комментарий. NULL если его нет |
| `sort_order` | INTEGER | Нет | Порядок сортировки. Новые задачи получают min-1 |
| `completion_event_id` | TEXT | Да | UUID события завершения родителя. Только для корневых задач |
| `auto_completed_by` | TEXT | Да | completion_event_id родителя, завершившего эту подзадачу |

### Семантика полей завершения

**`completion_event_id`:**
- Устанавливается при завершении корневой задачи
- Идентифицирует событие завершения
- Используется для точного восстановления подзадач при повторном открытии родителя

**`auto_completed_by`:**
- Устанавливается у подзадач при автоматическом завершении через родителя
- Содержит `completion_event_id` родителя
- NULL означает ручное завершение пользователем

### Календарные даты vs timestamps

- **`due_date`** — календарная дата без времени и таймзоны (YYYY-MM-DD)
- **`created_at`, `updated_at`, `completed_at`, `deleted_at`** — полные timestamp в RFC 3339 UTC

## Таблица `tabs`

### Поля

| Поле | Тип | NULL | Описание |
|------|-----|------|----------|
| `id` | TEXT | Нет | UUID v4, первичный ключ |
| `name` | TEXT | Нет | Название вкладки |
| `color` | TEXT | Нет | Цвет `#rrggbb` из палитры `TAB_COLORS`, назначается автоматически |
| `sort_order` | INTEGER | Нет | Порядок вкладок в полосе |
| `created_at` | TEXT | Нет | Время создания (RFC 3339 UTC) |
| `updated_at` | TEXT | Нет | Время последнего обновления (RFC 3339 UTC) |
| `deleted_at` | TEXT | Да | Время удаления вкладки |

Подзадача наследует вкладку родителя, поэтому дерево задач никогда не
разрывается между вкладками.

Удалённая вкладка остаётся в таблице с заполненным `deleted_at`, а её задачи
уходят в корзину. Строка нужна для внешнего ключа: без неё задачи остались бы
без вкладки, а откат не смог бы вернуть их туда же. Список вкладок отбирает
`deleted_at IS NULL`, а строка исчезает из базы, когда корзина опустеет и
ссылаться на неё будет некому.

## Таблица `settings`

### Поля

| Поле | Тип | NULL | Описание |
|------|-----|------|----------|
| `key` | TEXT | Нет | Ключ настройки (первичный ключ) |
| `value` | TEXT | Нет | Значение настройки |

### Известные ключи

| Ключ | Значение | Описание |
|------|----------|----------|
| `theme` | `system` / `light` / `dark` | Тема оформления |
| `show_completed` | `0` / `1` | Показывать завершённые задачи |
| `cascade_complete` | `0` / `1` | Завершение родителя завершает подзадачи |
| `auto_delete_days` | число / пусто | Через сколько дней закрытое дерево уходит в корзину. **По умолчанию выключена** (значение отсутствует) |
| `language` | `ru` / `en` | Язык интерфейса |
| `expand_subtasks` | `0` / `1` | Раскрывать подзадачи по умолчанию |
| `show_tab_counts` | `0` / `1` | Счётчики задач на вкладках |
| `newest_first` | `0` / `1` | Новые задачи сверху, иначе снизу |

## Таблица `schema_migrations`

### Поля

| Поле | Тип | NULL | Описание |
|------|-----|------|----------|
| `version` | INTEGER | Нет | Номер версии миграции (первичный ключ) |
| `name` | TEXT | Нет | Название миграции |
| `applied_at` | TEXT | Нет | Время применения (RFC 3339 UTC) |

## Индексы

| Индекс | Таблица | Поля | Тип | Назначение |
|--------|---------|------|-----|------------|
| `idx_tasks_active_roots` | tasks | `sort_order` | Partial (WHERE deleted_at IS NULL AND parent_id IS NULL) | Быстрый поиск активных корневых задач |
| `idx_tasks_subtasks` | tasks | `parent_id, sort_order` | Partial (WHERE deleted_at IS NULL AND parent_id IS NOT NULL) | Быстрый поиск подзадач |
| `idx_tasks_title` | tasks | `title` | Full | Поиск по названию |
| `idx_tasks_deleted` | tasks | `deleted_at` | Partial (WHERE deleted_at IS NOT NULL) | Поиск удалённых задач |
| `idx_tasks_by_tab` | tasks | `tab_id, sort_order` | Partial (WHERE deleted_at IS NULL) | Выборка задач вкладки |
| `idx_tasks_search_text` | tasks | `search_text` | Full | Регистронезависимый поиск по названию |
| `idx_tabs_order` | tabs | `sort_order` | Full | Порядок вкладок |

## Триггеры

### `task_no_third_level_insert`

```sql
CREATE TRIGGER task_no_third_level_insert
BEFORE INSERT ON tasks
WHEN NEW.parent_id IS NOT NULL
  AND EXISTS (SELECT 1 FROM tasks WHERE id = NEW.parent_id AND parent_id IS NOT NULL)
BEGIN
    SELECT RAISE(ABORT, 'only one subtask level is allowed');
END;
```

Блокирует вставку задач с третьим уровнем вложенности.

### `task_no_third_level_update`

```sql
CREATE TRIGGER task_no_third_level_update
BEFORE UPDATE OF parent_id ON tasks
WHEN NEW.parent_id IS NOT NULL
  AND EXISTS (SELECT 1 FROM tasks WHERE id = NEW.parent_id AND parent_id IS NOT NULL)
BEGIN
    SELECT RAISE(ABORT, 'only one subtask level is allowed');
END;
```

Блокирует обновление `parent_id` на третий уровень вложенности.

## Формат экспорта

Экспорт — это снимок **всего состояния приложения**, а не только видимого
списка. Формат один — JSON.

| Раздел | Что содержит |
|--------|--------------|
| `version` | Версия формата. Сейчас `2`. |
| `exportedAt` | Момент выгрузки, RFC 3339. |
| `tabs` | Все вкладки: `id`, `name`, `color`, `sortOrder`, даты, `deletedAt`. |
| `settings` | Все настройки: тема, язык, показ выполненных, каскад, раскрытие подзадач, счётчики вкладок, «новые сверху», авто-удаление. |
| `tasks` | **Все** задачи, включая выполненные и удалённые, с полями `parentId` (иерархия) и `tabId`. |

Поля задач сериализуются в `camelCase` (тот же контракт, что и для UI), поэтому
файл экспорта совпадает с моделью `src/lib/types.ts`.

Обратная совместимость: файл версии 1 (только `tasks`) импортируется как
раньше — `tabs` и `settings` необязательны.

```json
{
  "version": 2,
  "exportedAt": "2026-10-14T12:00:00.000Z",
  "tabs": [
    {
      "id": "8c1f0e0a-1f2b-4c3d-9e5f-0a1b2c3d4e5f",
      "name": "Работа",
      "color": "#4c6fff",
      "sortOrder": 0,
      "createdAt": "2026-10-14T10:00:00.000Z",
      "updatedAt": "2026-10-14T10:00:00.000Z"
    }
  ],
  "settings": {
    "theme": "dark",
    "showCompleted": true,
    "cascadeComplete": false,
    "autoDeleteDays": 30,
    "language": "ru",
    "expandSubtasks": true,
    "showTabCounts": true,
    "newestFirst": true
  },
  "tasks": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "parentId": null,
      "tabId": "8c1f0e0a-1f2b-4c3d-9e5f-0a1b2c3d4e5f",
      "title": "Купить продукты",
      "createdAt": "2026-10-14T10:00:00.000Z",
      "updatedAt": "2026-10-14T10:00:00.000Z",
      "completedAt": null,
      "deletedAt": null,
      "dueDate": "2026-10-16",
      "comment": null,
      "sortOrder": -1,
      "completionEventId": null,
      "autoCompletedBy": null
    },
    {
      "id": "550e8400-e29b-41d4-a716-446655440001",
      "parentId": "550e8400-e29b-41d4-a716-446655440000",
      "tabId": "8c1f0e0a-1f2b-4c3d-9e5f-0a1b2c3d4e5f",
      "title": "Молоко",
      "createdAt": "2026-10-14T10:01:00.000Z",
      "updatedAt": "2026-10-14T10:01:00.000Z",
      "completedAt": null,
      "deletedAt": null,
      "dueDate": null,
      "comment": null,
      "sortOrder": 1,
      "completionEventId": null,
      "autoCompletedBy": null
    }
  ]
}
```

### Почему только JSON

YAML и XML были в ранних сборках и удалены осознанно:

- `serde_yml` помечен как **небезопасный и неподдерживаемый** (RUSTSEC-2025-0068);
- `quick-xml` содержал **две известные уязвимости** (RUSTSEC-2026-0194 —
  квадратичное время работы, RUSTSEC-2026-0195 — неограниченное выделение
  памяти) на файлах, специально подобранных злоумышленником.

Импорт принимает файл откуда угодно, поэтому парсер, который никто не может
проверить, — это риск без выгоды. Один формат — один код, который нужно
понимать. `cargo audit` теперь проходит без исключений.

Файл в старом формате не примет��тся молча: импорт объяснит, что нужен JSON.

### Правила импорта

1. **Вкладки** сливаются по `id`: существующая обновляется, новая создаётся.
   Произвольный цвет из файла, которого нет в палитре, заменяется на палитровый.
2. **Задачи** сливаются по `id`. Ничего не удаляется: импорт может только
   добавить или обновить.
3. Подзадача, чей родитель не найден, становится **корневой** — иначе она была
   бы невидима.
4. **Настройки** применяются последними, если они есть в файле.
5. Формат — всегда JSON. Файл YAML или XML отклоняется с объяснением, а не с
   сырой ошибкой парсера.

## Локализация файла БД

| Платформа | Путь |
|-----------|------|
| macOS | `~/Library/Application Support/com.quita.app/quita.db` |
| Windows | `%APPDATA%\com.quita.app\quita.db` |
| Linux | `~/.local/share/com.quita.app/quita.db` |
