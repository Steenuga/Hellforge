# Публікація Hellforge 1.0.0 на GitHub

## 1. Завантаж вихідний код

У репозиторії `Steenuga/Hellforge` натисни **Add file → Upload files** і завантаж **вміст** папки `hellforge`, не саму ZIP-архівну оболонку. Збережи підпапки `src`, `src-tauri`, `.github/workflows`, `scripts`. При завантаженні через браузер приховані папки `.github` можуть не потрапити до перетягування: перевір, що файл `.github/workflows/windows.yml` справді є на GitHub. Не замінюй раніше оформлені README.md, README.uk.md та assets, якщо вони вже завантажені. Краще використовувати Git або GitHub Desktop для коректного завантаження всіх папок.

## 2. Запусти збірку

GitHub → **Actions** → **Build Hellforge for Windows** → **Run workflow** → **Run workflow**. Дочекайся завершення. Якщо збірка червона — відкрий невдалий крок і збережи лог. Якщо зелена — у виконанні workflow в розділі **Artifacts** завантаж `Hellforge-1.0.0-Windows-Installer`.

## 3. Перевір результат

Розпакуй артефакт на Windows, запусти інсталятор, перевір іконку, запуск, імпорт IWAD, створення профілю, запуск GZDoom, моди, мапи, генератор OBSIDIAN, теми й перемикання мов. Не публікуй реліз, якщо збірка неуспішна або застосунок не запускається.

## 4. Створи реліз

GitHub → **Releases** → **Draft a new release** → **Choose a tag** → створи `v1.0.0` від гілки `main`. Title: `Hellforge 1.0.0 — Initial Release`. Встав текст з `GITHUB_RELEASE_NOTES_EN.md`. Прикріпи до Assets перевірений інсталятор `.exe` із Actions. Натисни **Publish release**. Опція **Set as a pre-release** вимкнена для фінального релізу.

## Важливо

Цей ZIP — вихідний код, **не** інсталятор. Windows EXE можна отримати лише після успішної збірки. GitHub Actions може потребувати виправлень залежностей/коду; наявність workflow не гарантує успіху.
