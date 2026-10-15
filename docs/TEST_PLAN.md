# План тестирования QuiTa

## Обзор

Тестирование QuiTa разделено на три уровня:
1. **Unit-тесты Rust** — доменная логика и валидация
2. **Интеграционные тесты Rust** — работа с реальной базой данных
3. **Тесты TypeScript** — фронтенд-логика

## Актуальные числа

| Набор | Команда | Тестов |
|-------|---------|--------|
| Rust unit | `cargo test --manifest-path src-tauri/Cargo.toml` | 40 |
| Rust integration | то же | 74 |
| TypeScript | `pnpm test` | 115 |

Числа берутся из фактического прогона. Если они разошлись с таблицей —
значит, тесты забыли пересчитать, и это такое же расхождение кода и
текста, как неверное описание поведения.

Обе платформы проверены сборкой и запуском:

- **macOS (Apple Silicon)** — `pnpm tauri build`, приложение запускается, база создаётся
- **macOS (Intel)** — `pnpm tauri build --target x86_64-apple-darwin`, запуск под Rosetta
- **Linux (aarch64)** — сборка `.deb` и `.AppImage` в контейнере `node:22-bookworm`,
  установка `.deb` через `apt`, запуск под Xvfb + dbus, база создаётся
- **Windows** — собирается в CI (`.github/workflows/ci.yml`, `release.yml`)

## Unit-тесты Rust

### `src-tauri/src/domain/validation.rs`

| Тест | Что проверяет |
|------|---------------|
| `normalize_trims_and_rejects_empty` | Обрезка пробелов, отклонение пустых строк, поддержка Unicode |
| `due_date_accepts_real_calendar_dates` | Приём валидных дат, отклонение невозможных дат (2025-02-30, 2025-13-01), отклонение некалендарных форматов |

### `src-tauri/src/domain/rules.rs`

| Тест | Что проверяет |
|------|---------------|
| `completing_parent_auto_completes_children_under_one_event` | Завершение родителя завершает всех незавершённых детей под одним event_id |
| `completing_parent_keeps_manually_completed_children_untouched` | Ручные завершения детей не затрагиваются |
| `reopening_parent_reopens_only_auto_completed_children` | Повторное открытие родителя восстанавливает только автоматически завершённых детей |
| `reopening_subtask_leaves_the_completed_parent_closed` | Снятие галочки с подзадачи не открывает закрытого родителя |
| `reopening_a_parent_after_a_child_was_taken_back_out_respects_that_child` | Открытое пользователем остаётся открытым при открытии родителя |

## Интеграционные тесты Rust

Файл: `src-tauri/tests/integration_tests.rs`

### Создание и вложенность

| Тест | Что проверяет |
|------|---------------|
| `create_task_and_subtask` | Создание корневой задачи и подзадачи, правильные sort_order |
| `third_level_is_rejected_by_domain_logic` | Отклонение третьего уровня вложенности |
| `empty_and_whitespace_titles_are_rejected` | Отклонение пустых названий |
| `titles_are_trimmed` | Обрезка пробелов в названиях |

### Правила завершения

| Тест | Что проверяет |
|------|---------------|
| `completing_parent_completes_all_children_under_one_event` | Каскадное завершение подзадач с записью event_id |
| `reopening_parent_reopens_only_children_completed_by_that_event` | Точечное восстановление подзадач |
| `reopening_a_child_leaves_the_completed_parent_closed` | Частично выполнено вместо молчаливого открытия родителя |
| `completing_child_keeps_parent_open_and_is_recorded_as_manual` | Ручное завершение подзадачи не трогает родителя и не помечается как авто-завершение |
| `adding_subtasks_to_completed_parent_is_rejected` | Запрет добавления подзадач к завершённому родителю |

### Удаление и восстановление

| Тест | Что проверяет |
|------|---------------|
| `soft_delete_cascades_atomically` | Каскадное мягкое удаление |
| `restore_does_not_bring_back_independently_deleted_children` | Восстановление родителя не восстанавливает отдельно удалённых детей |
| `restore_of_subtask_with_deleted_parent_is_rejected` | Запрет восстановления подзадачи с удалённым родителем |

### Сроки

| Тест | Что проверяет |
|------|---------------|
| `due_date_roundtrips_as_calendar_date_without_tz_shift` | Календарные даты не зависят от таймзоны |
| `changing_parent_due_date_does_not_touch_children` | Срок родителя не влияет на детей |
| `invalid_due_dates_are_rejected` | Отклонение невалидных дат |

### Сортировка

| Тест | Что проверяет |
|------|---------------|
| `new_roots_go_to_the_top` | Новые задачи появляются сверху |
| `order_survives_connection_restart` | Порядок сохраняется между сессиями |
| `move_task_swaps_sort_order_with_neighbor` | Перемещение задач меняет sort_order |

### Поиск

| Тест | Что проверяет |
|------|---------------|
| `search_excludes_deleted_and_completed_tasks_by_default` | Поиск исключает удалённые и завершённые |
| `search_matches_subtasks_with_parent_context` | Поиск по подзадачам с контекстом родителя |
| `search_escapes_like_wildcards` | Экранирование % и _ |

### Экспорт

| Тест | Что проверяет |
|------|---------------|
| `export_contains_all_fields_and_preserves_links` | Экспорт содержит все поля и связи |

### Валидация (зеркало unit-тестов)

| Тест | Что проверяет |
|------|---------------|
| `validation_helpers_match_their_specs` | Соответствие валидации спецификации |
| `toggle_plan_children_are_filtered_correctly` | Фильтрация детей в плане завершения |

## Тесты TypeScript (vitest)

### `tests/taskValidation.test.ts`

| Тест | Что проверяет |
|------|---------------|
| `normalizeTitle` — trims whitespace | Обрезка пробелов |
| `normalizeTitle` — rejects empty | Отклонение пустых строк |
| `normalizeTitle` — preserves unicode | Поддержка Unicode |
| `isValidDueDate` — accepts real dates | Приём валидных дат |
| `isValidDueDate` — rejects impossible dates | Отклонение невозможных дат |
| `isValidDueDate` — rejects non-calendar formats | Отклонение некалендарных форматов |
| `todayLocalDateString` | Форматирование локальной даты |

### `tests/taskRules.test.ts`

| Тест | Что проверяет |
|------|---------------|
| `buildTaskTree` — sorts by sortOrder | Сортировка по sortOrder |
| `getSubtaskStats` — counts completed and total | Подсчёт статистики подзадач |
| `getSubtaskStats` — handles no subtasks | Обработка пустого списка |
| `formatProgress` — formats done/total | Форматирование прогресса |
| `formatProgress` — returns null when empty | Пустой прогресс |
| `isOverdue` — detects overdue | Определение просрочки |
| `isOverdue` — treats today as not overdue | Сегодняшняя дата не просрочена |
| `isOverdue` — treats missing due dates | Отсутствие срока |
| `canHaveSubtasks` — allows active roots | Активные корневые задачи |
| `canHaveSubtasks` — forbids subtasks | Запрет подзадач |
| `canHaveSubtasks` — forbids completed | Запрет завершённых |

### `tests/taskItem.test.ts`

SSR-рендер компонента задачи — проверяет контракт «незавершённая задача не выглядит завершённой»:

| Тест | Что проверяет |
|------|---------------|
| `renders an open task as unchecked` | `aria-checked="false"`, нет класса `completed` |
| `renders a completed task as checked` | Завершённая задача отмечена |
| `marks an overdue task and shows its calendar date` | Подсветка просрочки и дата в формате YYYY-MM-DD |
| `offers subtask creation for active roots and shows progress` | Прогресс `1/2` и кнопка подзадачи у корневой задачи |
| `does not offer subtasks, reordering or nested add for subtasks` | У подзадачи нет кнопки подзадач и переупорядочивания |

### `tests/taskStore.test.ts`

Сторы импортируются по-настоящему (Tauri API замокан) — это поймает регрессии
вроде runes в модуле без расширения `.svelte.ts`:

| Тест | Что проверяет |
|------|---------------|
| `adds a new root task to the top of the list` | Вызов `create_task` и вставка наверх |
| `adds a subtask to the matching parent` | Подзадача попадает к нужному родителю |
| `removes the node after soft delete` | Узел исчезает после `soft_delete_task` |
| `toggle asks the backend and reloads the list` | Toggle вызывает команду и перезагружает список |
| `keeps completed tasks visible when the setting is on` | Настройка показа завершённых учитывается |

## Ручное тестирование

### Трей

| Сценарий | Ожидаемое поведение |
|----------|---------------------|
| Левый клик по иконке в трее | Окно показывается/скрывается |
| Правый клик по иконке в трее | Открывается меню |
| Пункт меню «Открыть QuiTa» | Окно показывается и фокусируется |
| Пункт меню «Выйти» | Приложение завершается |
| Крестик в окне | Окно скрывается, приложение остаётся в трее |

### Горячая клавиша

| Сценарий | Ожидаемое поведение |
|----------|---------------------|
| Нажатие Cmd+Shift+Q (macOS) / Ctrl+Shift+Q (Windows/Linux) из другого приложения | Окно показывается, фокус на поле ввода |
| Повторное нажатие, когда окно видимо | Окно скрывается |
| Отсутствие прав доступа (macOS) | Ошибка логируется, приложение работает через трей |

### Автозапуск

| Сценарий | Ожидаемое поведение |
|----------|---------------------|
| Перезагрузка системы | Приложение не запускается автоматически (не реализовано в MVP) |

## Известные ограничения

- **E2E-тесты не автоматизированы** — нет Playwright или аналогов
- **Тесты горячей клавиши** — только ручное тестирование
- **Тесты трея** — только ручное тестирование
- **Тесты на разных платформах** — ручное тестирование на macOS, Windows, Linux

## CI

GitHub Actions (`.github/workflows/ci.yml`):
- **Frontend**: pnpm check, pnpm test, pnpm build
- **Rust**: cargo fmt --check, cargo clippy -- -D warnings, cargo test
- **Сборка**: macOS (Apple Silicon), Windows, Linux

## Фактические результаты проверок

Проверено на macOS 15 (Apple Silicon, Rust 1.99, Node 26, pnpm 12):

| Проверка | Команда | Результат |
|----------|---------|-----------|
| Типы и линт UI | `pnpm check` | 0 ошибок, 0 предупреждений |
| Тесты UI | `pnpm test` | 18 из 18 пройдено |
| Форматирование Rust | `cargo fmt --check` | без замечаний |
| Линтер Rust | `cargo clippy --all-targets -- -D warnings` | без замечаний |
| Тесты Rust | `cargo test` | 7 unit + 24 интеграционных пройдено |
| Сборка фронтенда | `pnpm build` | успешно (~67 kB JS, ~11 kB CSS до gzip) |
| Сборка приложения | `pnpm tauri build` | `QuiTa.app` (12.88 MiB) и `.dmg` для aarch64 |
| Smoke-тест запуска | запуск собранного приложения | БД создаётся автоматически, схема и триггеры на месте, данные переживают перезапуск |
| Ограничение третьего уровня | прямой SQL в обход приложения | вставка подзадачи под подзадачей отклонена триггером БД |

Что **не** проверено автоматически и требует ручного тестирования:

- визуальный вид интерфейса и поведение трея/меню-бара (нужен интерактивный доступ к рабочему столу);
- фактическое срабатывание глобальной горячей клавиши на каждой ОС;
- поведение на Windows и Linux — собирается в CI, но вручную не проверялось;
- диалог экспорта JSON в реальном окне ОС (логика формирования файла покрыта тестом `export_contains_all_fields_and_preserves_links`).

## Тесты инвариантов, добавленные после аудита

Набор добавлен по результатам ревью кода и закрывает найденные классы дефектов:

| Тест | Что защищает |
|------|--------------|
| `error_messages_serialize_as_strings` | ошибки доходят до UI строкой, а не `[object Object]` |
| `moving_a_subtask_to_another_tab_moves_its_parent` | дерево задач никогда не разрывается между вкладками |
| `purge_refuses_a_task_that_is_not_deleted` | необратимое удаление доступно только для мягко удалённых задач |
| `deleting_a_tab_never_targets_itself` | удаление вкладки не падает на внешнем ключе и не теряет задачи |
| `a_task_never_ends_up_without_a_tab` | задача без вкладки не пропадает из списков |
| `empty_tab_name_is_rejected_instead_of_silently_defaulted` | единый контракт ошибок для имён вкладок |
| `count_subtasks_includes_completed_and_hidden_children` | подтверждение удаления показывает настоящее число подзадач |
| `overlong_titles_are_rejected_at_the_boundary` | лимит длины дублируется в Rust, а не только в `maxlength` |
| `fill` (Vitest) | заголовок с `$&` / `` $` `` не портит aria-label |

## Ручные проверки после аудита

- [x] macOS M1: сборка, запуск, трей-иконка, квадратные чекбоксы
- [x] macOS Intel: сборка и запуск под Rosetta
- [x] Linux: сборка `.deb`/`.AppImage`, установка `.deb`, запуск под Xvfb + dbus
- [ ] Windows: установка `.msi`, трей, горячая клавиша — выполняется вручную после первого релиза
- [ ] Перетаскивание подзадачи на вкладку переносит её родителя (проверка вручную)
