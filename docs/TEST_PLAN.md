# План тестирования QuiTa

## Обзор

Тестирование QuiTa разделено на четыре уровня:
1. **Unit-тесты Rust** — доменная логика и валидация
2. **Интеграционные тесты Rust** — работа с реальной базой данных
3. **Тесты документации Rust** — ссылки в README ведут в существующие файлы
4. **Тесты TypeScript** — фронтенд-логика

## Актуальные числа

| Набор | Команда | Тестов |
|-------|---------|--------|
| Rust unit | `cargo test --manifest-path src-tauri/Cargo.toml` | 40 |
| Rust integration | то же | 105 |
| Rust документация | то же (`tests/documentation.rs`) | 4 |
| TypeScript | `pnpm test` | 152 |

Числа берутся из фактического прогона. Если они разошлись с таблицей —
значит, тесты забыли пересчитать, и это такое же расхождение кода и
текста, как неверное описание поведения.

Платформы проверены сборкой и запуском:

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
| `toggling_a_subtask_never_touches_the_parent_in_either_direction` | Ни завершение, ни открытие подзадачи не трогают родителя |
| `cascade_disabled_leaves_children_untouched` | Без каскада галочка родителя меняет только его самого |
| `cascade_disabled_mean_reopening_restores_nothing` | Без каскада открытие родителя не восстанавливает подзадачи |

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
| `reopening_a_parent_after_a_child_was_taken_back_out_respects_that_child` | Открытое пользователем остаётся открытым при открытии родителя |
| `completing_child_keeps_parent_open_and_is_recorded_as_manual` | Ручное завершение подзадачи не трогает родителя и не помечается как авто-завершение |
| `completed_parent_can_receive_new_subtasks` | Завершённый родитель подзадачи принимает — строка читается как «частично выполнено» |
| `cascade_disabled_completes_only_the_parent` | Без каскада подзадачи остаются как были |

### Удаление и восстановление

| Тест | Что проверяет |
|------|---------------|
| `soft_delete_cascades_atomically` | Каскадное мягкое удаление |
| `restoring_a_parent_brings_back_children_deleted_before_it` | Восстановление родителя возвращает и его мягко удалённых подзадач |
| `restore_of_subtask_with_deleted_parent_promotes_it` | Подзадача с удалённым родителем восстанавливается как корневая |
| `trash_lists_subtasks_and_restore_brings_them_back` | Корзина показывает подзадачи рядом с родителем |
| `restoring_a_subtask_on_its_own_promotes_it_to_a_root_task` | Отдельное восстановление подзадачи поднимает её на верхний уровень |

### Автоочистка завершённых

| Тест | Что проверяет |
|------|---------------|
| `purge_moves_a_finished_tree_to_the_trash` | Закрытое дерево уходит в корзину вместе с подзадачами, а не исчезает |
| `purge_waits_for_the_last_closure_in_the_tree` | Отсчёт идёт от последнего завершения в дереве |
| `purge_leaves_a_tree_with_open_work_alone` | Незавершённая подзадача удерживает дерево, закрытая подзадача не уходит от открытого родителя |

### Срок в корзине

| Тест | Что проверяет |
|------|---------------|
| `purge_trash_by_age_removes_an_expired_tree` | Просроченное дерево удаляется из базы совсем, а свежая задача остаётся |
| `purge_trash_by_age_waits_for_the_latest_deletion_in_the_tree` | Отсчёт идёт от последнего удаления в дереве, а не от первого |
| `purge_trash_by_age_treats_a_deleted_subtask_on_its_own_as_its_own_unit` | Удалённая отдельно подзадача уходит сама, живой родитель остаётся |
| `purge_trash_by_age_is_off_until_it_is_asked_for` | Настройка пуста по умолчанию: срок, о котором не просили, ничего не удаляет |
| `the_trash_retention_is_stored_and_cleared_again` | Срок сохраняется и снимается обратно |

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
| `search_finds_cyrillic_whatever_the_case` | Регистр кириллицы не мешает поиску |
| `search_finds_subtasks_in_any_case_too` | То же для подзадач |
| `tasks_written_before_the_search_column_are_still_findable` | Дозаполнение `search_text` на старте |

### Экспорт

| Тест | Что проверяет |
|------|---------------|
| `export_contains_all_fields_and_preserves_links` | Экспорт содержит все поля и связи |

### Валидация (зеркало unit-тестов)

| Тест | Что проверяет |
|------|---------------|
| `validation_helpers_match_their_specs` | Соответствие валидации спецификации |
| `toggle_plan_children_are_filtered_correctly` | Фильтрация детей в плане завершения |

### Документация (`src-tauri/tests/documentation.rs`)

| Тест | Что проверяет |
|------|---------------|
| `every_referenced_screenshot_exists` | Все картинки, на которые ссылаются README, лежат в репозитории |
| `no_screenshot_is_left_unused` | Нет скриншотов, на которые никто не ссылается |
| `screenshots_are_small_enough_to_load_in_a_readme` | Скриншоты не утяжеляют страницу репозитория |

## Тесты TypeScript (vitest)

Девятнадцать файлов, 141 тест. Ниже перечислены все наборы.

### `tests/taskValidation.test.ts`

| Тест | Что проверяет |
|------|---------------|
| `normalizeTitle` — trims whitespace from both ends | Обрезка пробелов |
| `normalizeTitle` — rejects empty and whitespace-only input | Отклонение пустых строк |
| `normalizeTitle` — preserves unicode content | Поддержка Unicode |
| `isValidDueDate` — accepts real calendar dates | Приём валидных дат |
| `isValidDueDate` — rejects impossible dates | Отклонение невозможных дат |
| `isValidDueDate` — rejects non-calendar formats | Отклонение некалендарных форматов |
| `todayLocalDateString` — formats a local date as YYYY-MM-DD | Форматирование локальной даты |

### `tests/dueDate.test.ts`

| Тест | Что проверяет |
|------|---------------|
| `moves a date forward and backward without timezone drift` | Сдвиг даты без сдвига таймзоны |
| `picks today when nothing is selected yet` | Пустой выбор начинается с сегодня |
| `adds one day to an existing date` / `crosses a month boundary` | Переход через границу месяца |
| `subtracts a day` | Шаг назад |
| `never goes before today` | Нижняя граница |
| `maps a number of days to a calendar date` | Относительные даты |
| `accepts a calendar date and rejects an impossible one` | Валидация |

### `tests/taskRules.test.ts`

| Тест | Что проверяет |
|------|---------------|
| `buildTaskTree` — sorts nodes by sortOrder | Сортировка по sortOrder |
| `getSubtaskStats` — counts completed and total subtasks | Подсчёт статистики подзадач |
| `getSubtaskStats` — handles no subtasks | Обработка пустого списка |
| `formatProgress` — formats done/total including the task itself | Форматирование прогресса |
| `formatProgress` — returns null when there are no subtasks | Пустой прогресс |
| `isOverdue` — detects overdue calendar dates | Определение просрочки |
| `isOverdue` — treats today and future dates as not overdue | Сегодняшняя дата не просрочена |
| `isOverdue` — treats missing due dates as not overdue | Отсутствие срока |
| `canHaveSubtasks` — allows active root tasks | Активные корневые задачи |
| `canHaveSubtasks` — forbids subtasks (only one level) | Запрет второго уровня |
| `canHaveSubtasks` — allows completed roots to receive more subtasks | Завершённый родитель подзадачи принимает |

### `tests/taskItem.test.ts`

SSR-рендер компонента задачи — проверяет контракт «незавершённая задача не выглядит завершённой»:

| Тест | Что проверяет |
|------|---------------|
| `renders an open task as unchecked` | `aria-checked="false"`, нет класса `completed` |
| `renders a completed task as checked` | Завершённая задача отмечена |
| `marks an overdue task and shows its calendar date` | Подсветка просрочки и дата в формате YYYY-MM-DD |
| `offers subtask creation for roots and counts the parent in progress` | Прогресс и кнопка подзадачи у корневой задачи |
| `marks a completed task with open subtasks as partially done` | Состояние «частично выполнено» |
| `does not offer subtasks, reordering or nested add for subtasks` | У подзадачи нет кнопки подзадач и переупорядочивания |
| `exposes every limit the repository enforces` | Лимиты приходят с бэкенда, а не из константы |

### `tests/taskStore.test.ts`

Сторы импортируются по-настоящему (Tauri API замокан) — это поймает регрессии
вроде runes в модуле без расширения `.svelte.ts`:

| Тест | Что проверяет |
|------|---------------|
| `adds a new root task to the top of the list` | Вызов `create_task` и вставка наверх |
| `adds a subtask to the matching parent` | Подзадача попадает к нужному родителю |
| `removes the node after soft delete` | Узел исчезает после `soft_delete_task` |
| `toggle applies the outcome in place instead of reloading the list` | Toggle применяет ответ бэкенда точечно |
| `brings a reopened parent back into the list with its subtasks` | Открытый родитель возвращается в список вместе с подзадачами |
| `reparent moves a task under another parent` | Перенос задачи к другому родителю |
| `moveToTab sends the task to another tab` | Перенос задачи в другую вкладку |
| `purges expired completed tasks when the retention setting is on` | Автоочистка по настройке хранения |
| `sweeps the trash by age on every start, setting or no setting` | Подметание корзины по сроку вызывается всегда: настройку читает команда |
| `passes the cascade setting from the store` | Настройка каскада доходит до команды |
| `keeps completed tasks visible when the setting is on` | Настройка показа завершённых учитывается |

### `tests/capture.test.ts`

| Тест | Что проверяет |
|------|---------------|
| `saves the trimmed title` | `Enter` создаёт задачу |
| `does nothing on an empty or blank field` | Пустое поле не создаёт задачу |
| `ignores a repeated Enter while a save is running` | Защита от повторного создания |
| `can be used any number of times in a row` | Поле остаётся готовым к следующей задаче |
| `cancels: nothing typed is created` | `Escape` ничего не создаёт |
| `saves what is typed instead of losing it` | Уход с поля сохраняет набранное |
| `closes an empty field without touching the database` | Пустое поле просто закрывается |
| `never leaves the field stuck open` | Поле всегда закрывается |

### `tests/dragZones.test.ts`

| Тест | Что проверяет |
|------|---------------|
| `treats the middle half of a row as the nesting zone` | Средняя половина строки — вложение |
| `treats the top and bottom quarters as insertion zones` | Верх и низ строки — вставка |
| `never nests when the target cannot adopt subtasks` | Подзадача не принимает подзадачу |
| `inserts above when released in the top part, below in the bottom part` | Сторона сброса решает позицию |
| `moves a subtask up when dropped in the upper half of another one` | Перестановка подзадач вверх |
| `moves a subtask down when dropped in the lower half` | Перестановка подзадач вниз |
| `can put a subtask first` | Подзадача первой в группе |
| `places a task that joins the group from outside where the line is` | Позиция берётся из линии вставки |
| `finds the row the pointer is over` | Строка под указателем |
| `appends when the pointer is below the last row` | Вставка в конец списка |
| `puts it last, not second-to-last` | Крайняя позиция достижима |

### `tests/scroll.test.ts`

| Тест | Что проверяет |
|------|---------------|
| `puts the row 8px below the top of the container` | Позиция новой строки |
| `keeps the current scroll as the base, not the page offset` | Базой служит прокрутка контейнера |
| `never scrolls above the start` | Верхняя граница |
| `scrolls back up when the row is above the viewport` | Доскролл назад |
| `honours a custom inset` | Произвольный отступ |
| `returns the first row when new tasks go on top` | Новая строка сверху |
| `returns the last row when new tasks go to the bottom` | Новая строка снизу |

### `tests/tabStrip.test.ts`

| Тест | Что проверяет |
|------|---------------|
| `turns a vertical wheel gesture into horizontal movement` | Вертикальный жест листает вкладки |
| `keeps a trackpad horizontal gesture as is` | Горизонтальный жест не переворачивается |
| `scrolls and clamps at both ends` | Границы прокрутки |
| `shows nothing when all tabs fit` | Нет полосы прокрутки, когда всё влезло |
| `shows the right fade while there is more to the right` | Правый градиент |
| `shows both fades in the middle` | Градиенты с обеих сторон |
| `hides the right fade at the very end` | Правый градиент в конце |
| `moves roughly one screen per click` | Шаг кнопки прокрутки |

### `tests/trash.test.ts`

| Тест | Что проверяет |
|------|---------------|
| `places every subtask right below its deleted parent` | Подзадача видна под родителем |
| `reports how many subtasks each parent carries` | Счётчик подзадач в корзине |
| `keeps an orphaned subtask visible as a top-level row` | Подзадача без родителя не пропадает |
| `sorts siblings by their order instead of the arrival order` | Порядок берётся из `sort_order` |
| `does not lose a task when the data is malformed` | Устойчивость к битым данным |
| `handles an empty trash` | Пустая корзина |
| `never removes anything while the setting is empty` | Пустая настройка срока не удаляет ничего |
| `counts the days down from the moment the task was thrown away` | Подпись считает дни от даты удаления |
| `treats an overdue row as leaving on the next launch` | Просроченная строка уходит при следующем запуске |
| `does not invent a deadline from a missing or broken timestamp` | Битая дата не превращается в обещание |

### `tests/text.test.ts`

| Тест | Что проверяет |
|------|---------------|
| `substitutes a placeholder` | Подстановка `{name}` |
| `leaves unknown placeholders untouched` | Неизвестный плейсхолдер остаётся как есть |
| `does not expand dollar sequences in the value` | Значение не разворачивается как в `String.replace` |
| `handles empty values` | Пустое значение |

### `tests/i18n.test.ts`

| Тест | Что проверяет |
|------|---------------|
| `switches language at runtime` | Смена языка на лету |
| `reports changes to reactive readers` | Реактивность словаря |
| `translates every key used by the UI` | Полнота словарей |
| `has no stale English-only keys` | Нет ключей только в одном языке |
| `keeps the same placeholders in both languages` | Плейсхолдеры совпадают в обоих словарях |
| `falls back to the key when a translation is missing` | Отсутствующий перевод не ломает UI |

### `tests/updateStore.test.ts`

| Тест | Что проверяет |
|------|---------------|
| `starts hidden and quiet` | Баннер скрыт до проверки обновления |
| `is only "available" while a release is waiting` | Состояние доступного обновления |
| `computes progress only when the total size is known` | Прогресс только при известном размере |
| `forgets everything when dismissed` | Сброс после закрытия баннера |

### `tests/bumpVersion.test.ts`

| Тест | Что проверяет |
|------|---------------|
| `raises the version in all three files at once` | Версия поднимается во всех файлах |
| `accepts a tag with a leading v` | Тег с префиксом `v` |
| `finds the number behind any tag prefix` | Разбор префикса тега |
| `refuses anything without a version in it` | Отказ для строки без версии |
| `leaves everything alone when the version already matches` | Повторный запуск безопасен |
| `never changes files in --check mode` | Режим проверки ничего не пишет |
| `keeps the rest of each file intact` | Остальное содержимое файлов не меняется |

## Ручное тестирование

### Трей

| Сценарий | Ожидаемое поведение |
|----------|---------------------|
| Левый клик по иконке в трее | Окно показывается и фокусируется |
| Повторный левый клик при видимом окне | Окно снова получает фокус, скрытия не происходит |
| Правый клик по иконке в трее | Открывается меню |
| Пункт меню «Open QuiTa / Открыть QuiTa» | Окно показывается и фокусируется |
| Пункт меню «About / О программе» | Окно показывается, открывается диалог о программе |
| Пункт меню «Quit / Выйти» | Приложение завершается |
| Крестик в окне | Окно скрывается, приложение остаётся в трее |

### Горячая клавиша

| Сценарий | Ожидаемое поведение |
|----------|---------------------|
| ⌘⇧Q на macOS, Win+Alt+Q на Windows, Ctrl+Alt+Q на Linux из другого приложения | Окно показывается, фокус на поле ввода |
| Повторное нажатие при видимом окне | Окно снова получает фокус, скрытия не происходит |
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
- **Что изменилось**: сравнивает коммит с предыдущим и решает, что запускать
- **Frontend**: pnpm check, pnpm test, pnpm build, pnpm audit
- **Rust**: cargo fmt --check, cargo clippy -- -D warnings, cargo test, cargo audit
- **Сборка**: macOS (Apple Silicon), Windows, Linux
- **Разбор скриптов**: `bash -n scripts/install.sh`, парсер PowerShell для `install.ps1`
- **Смоук-тест Windows**: установка MSI, запуск, проверка базы, удаление

### Смоук-тест на Windows

Сборка доказывает только то, что отработал сборщик. Ставится ли MSI, лежит ли
исполняемый файл там, где Windows его ждёт, и переживает ли приложение
собственный старт — три отдельных вопроса, и у каждого на Windows своя
способность сломаться. Поэтому после сборки задача ставит свежий MSI,
запускает приложение и ждёт десять секунд:

- `msiexec` обязан вернуть 0;
- исполняемый файл обязан лежать в одном из четырёх стандартных мест. Путь
  намеренно не зашит: версия Tauri его уже двигала, а молча пропавший путь
  превратил бы проверку в ничего не проверяющую;
- процесс должен быть жив, иначе приложение упало при старте;
- база `quita.db` должна появиться — она пишется при первом запуске, значит
  инициализация прошла и хранилище открыто, а не висит пустое окно;
- после проверки приложение удаляется: установщик, который нельзя снести,
  делает следующий прогон проверкой пропатченной машины.

Чего тест не ловит: значок в трее, сочетание клавиш, разворот окна. Это
проверяется только вручную.

## Фактические результаты проверок

Прогоны тестов сделаны на macOS (Apple Silicon, Rust 1.99, Node 26, pnpm 12):

| Проверка | Команда | Результат |
|----------|---------|-----------|
| Тесты UI | `pnpm test` | 152 из 152 пройдено (20 файлов) |
| Тесты Rust | `cargo test` | 40 unit + 105 интеграционных + 4 на документацию |

Более ранний прогон, на котором сняты остальные строки таблицы:

| Проверка | Команда | Результат |
|----------|---------|-----------|
| Типы и линт UI | `pnpm check` | 0 ошибок, 0 предупреждений |
| Форматирование Rust | `cargo fmt --check` | без замечаний |
| Линтер Rust | `cargo clippy --all-targets -- -D warnings` | без замечаний |
| Сборка фронтенда | `pnpm build` | успешно (~67 kB JS, ~11 kB CSS до gzip) |
| Сборка приложения | `pnpm tauri build` | `QuiTa.app` (17.9 МБ) и `.dmg` (5.8 МБ) для aarch64 |
| Smoke-тест запуска | запуск собранного приложения | БД создаётся автоматически, схема и триггеры на месте, данные переживают перезапуск |
| Ограничение третьего уровня | прямой SQL в обход приложения | вставка подзадачи под подзадачей отклонена триггером БД |

Что **не** проверено автоматически и требует ручного тестирования:

- визуальный вид интерфейса и поведение трея/меню-бара (нужен интерактивный доступ к рабочему столу);
- фактическое срабатывание глобальной горячей клавиши на каждой ОС;
- поведение на Windows и Linux — собирается в CI, но вручную не проверялось;
- диалог экспорта JSON в реальном окне ОС (логика формирования файла покрыта тестом `export_contains_all_fields_and_preserves_links`).

## Числа тестов проверяет код

Числа выше не выписаны руками и не защищены от расхождения. Тест
`the_test_counts_in_the_docs_match_the_code` в `tests/documentation.rs` обходит
дерево исходников и требует, чтобы `TEST_PLAN.md` и `README` называли ровно
то число, которое есть на самом деле.

Причина отдельного теста: план писал «44 теста», имея 84, а README повторял
план — два документа соглашались между собой и оба были неправы. Согласие
документов между собой не считается проверкой.

Счётчики считают по строкам, а не подстрокой, иначе `taskStore.init()`
посчиталcя бы за `it(`. Обход дерева, а не список имён: перечислять 84 теста
вручную бессмысленно, список устареет первым же новым.

## Тесты инвариантов, добавленные после аудита

Набор добавлен по результатам ревью кода и закрывает найденные классы дефектов:

| Тест | Что защищает |
|------|--------------|
| `error_messages_serialize_as_strings` | ошибки доходят до UI строкой, а не `[object Object]` |
| `moving_a_subtask_to_another_tab_moves_its_parent` | дерево задач никогда не разрывается между вкладками |
| `purge_refuses_a_task_that_is_not_deleted` | необратимое удаление доступно только для мягко удалённых задач |
| `deleting_a_tab_takes_its_tasks_to_the_trash_and_keeps_the_last_one` | задачи вкладки уходят в корзину, а соседняя остаётся пустой |
| `undoing_a_tab_deletion_brings_the_tab_and_its_tasks_back` | откат возвращает вкладку и ровно те задачи, что унесла |
| `a_deleted_tabs_tasks_keep_pointing_at_it` | удалённая вкладка не отдаёт задачи соседней |
| `restoring_a_task_from_a_deleted_tab_brings_the_tab_back` | восстановление задачи возвращает и вкладку |
| `emptying_the_trash_drops_tabs_nothing_points_at` | опустевшая вкладка удаляется из базы |
| `the_last_visible_tab_survives_deleted_ones` | удалённые вкладки не мешают удалить последнюю |
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
