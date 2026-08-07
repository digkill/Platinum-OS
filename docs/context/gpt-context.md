# Контекст из ChatGPT

> Историческая выжимка ChatGPT. Не использовать как source of truth.
> Актуальные правила: `AGENTS.md`, `.cursor/rules/`, `dev-ai.md`, `board.toml`.
> Workspace уже на `platinum-*`. Ubuntu Base 26.04 LTS. Первая плата —
> Orange Pi Zero 3W (A733 / sun60iw2 / 12 GiB), не Zero 3 (H618).
> Armbian BSP: pin `a7f3a943…`, board `orangepizero3w`, branch `vendor`.

**Источник:** [Обновление Ubuntu 22.04 до 24.04](https://chatgpt.com/share/6a63b691-a298-83eb-b45c-293cd362882c)

Этот файл — сжатая выжимка длинного чата. Используй `@docs/context/gpt-context.md` в Cursor, чтобы агенты продолжали работу с того же контекста.

---

## Часть 1. Проблема: Orange Pi Zero 3W и Ubuntu 24.04

### Устройство

- Плата: **Orange Pi Zero 3W**
- Образ: **Orange Pi 1.0.0 Jammy** (кастомный, не чистая Canonical Ubuntu)
- Userspace: Ubuntu **22.04.5 LTS** (jammy)
- Ядро: `6.6.98-sun60iw2` — vendor kernel Orange Pi (Allwinner H618), не ядро Canonical
- Зеркало: `http://repo.huaweicloud.com/ubuntu-ports`

### Симптом

`sudo do-release-upgrade` падает:

```text
authenticate 'noble.tar.gz' against 'noble.tar.gz.gpg'
Authentication failed
```

Логи `/var/log/dist-upgrade/` не создаются — ошибка на этапе проверки подписи, до распаковки `noble.tar.gz`.

Дополнительные признаки кастомного образа:

- `ubuntu-keyring` вместо ожидаемого `ubuntu-archive-keyring`
- изменённый или несовместимый `ubuntu-release-upgrader`

### Вывод

**In-place upgrade 22.04 → 24.04 на vendor-образе Orange Pi не поддерживается.** Это не баг сети или времени — это несовместимость кастомной сборки с механизмом `do-release-upgrade`.

### Рекомендации из чата (для текущей платы)

| Вариант | Описание | Риск |
|--------|----------|------|
| ⭐ Armbian Ubuntu 24.04 Noble | Готовые образы для Orange Pi Zero, ядро Armbian `current`, нормальный `apt` | Низкий |
| Orange Pi Ubuntu 24.04 | Официальный vendor-образ, если доступен для Zero 3W | Низкий |
| Ручная замена jammy → noble | Почти наверняка сломает bootloader, DTB, ядро, драйверы | Высокий — не советовать |

Для Docker, Python, ROS, AI и сервисов предпочтительнее **Armbian Ubuntu 24.04 Noble**.

---

## Часть 2. Рождение проекта Platinum OS One

После обсуждения апгрейда чат перешёл к идее: **не зависеть от vendor-образов**, а собрать свою систему.

### Название и цель

**Platinum OS One** — минимальная ARM Linux-платформа на **Ubuntu Base**, для robotics, AI, серверов и embedded.

Изначально обсуждался Python (Typer), затем принято решение перейти на **Rust** с обучающим подходом: понимать каждую конструкцию, а не копировать код.

### Ключевые принципы

1. **Строим не образ, а фабрику** — `build()` → `.img`, как конвейер (Armbian Build, Yocto, Buildroot).
2. **Плата — это данные, не логика** — никаких `if board == "orangepi"` в коде сборки.
3. **Build-система универсальна** — не знает про Orange Pi; знает только этапы pipeline.
4. **BSP отдельно** — конфигурация платы в `boards/<name>/`.
5. **Маленькие осмысленные коммиты** — каждый commit улучшает систему.
6. **Не форк Armbian/Ubuntu** — дистрибутив на Ubuntu Base + собственный BSP + свои пакеты.

### Целевая архитектура образа

```text
Platinum OS One
├── Ubuntu Base 26.04 LTS (Canonical, Resolute Raccoon)
├── Linux Kernel (Armbian / vendor / mainline — per board)
├── U-Boot
├── Device Tree
├── Firmware
├── platinum-tools
├── platinum-update
├── platinum-config
└── platinum-agent
```

Стек для Orange Pi Zero 3W (решение из чата):

```text
Ubuntu Base 26.04 LTS
    + Armbian U-Boot
    + Armbian Device Tree
    + Armbian Kernel
    + Armbian firmware
```

Не брать готовый образ Armbian — **собрать свой** через build-систему.

### Структура образа на SD/eMMC

```text
OrangePiZero3W.img
├── U-Boot (Armbian)
├── Boot
│   ├── Image
│   ├── initrd
│   ├── dtb
│   └── extlinux.conf
└── RootFS
    └── Ubuntu Base 26.04
```

---

## Часть 3. Архитектура build-системы (Rust)

### Слои приложения

```text
main.rs
  ↓
CLI
  ↓
BuildEngine        ← ещё не реализован
  ↓
Pipeline
  ↓
Stages
```

### Rust workspace (текущий репозиторий)

```text
platinum/
├── crates/cli         # CLI, точка входа
├── crates/build       # Context, BuildEngine (в разработке)
├── crates/pipeline    # Pipeline + Stage trait
├── crates/board       # чтение board.toml
├── crates/rootfs      # сборка rootfs из Ubuntu Base
├── crates/image       # финальный .img
├── crates/downloader  # загрузка артефактов
└── crates/logger      # логирование
```

### Pipeline stages (план)

```text
prepare()
  ↓
download()
  ↓
build_bootloader()
  ↓
build_kernel()
  ↓
build_rootfs()
  ↓
make_image()
```

### Конфигурация платы (`boards/orangepi-zero3w/`)

Пример `board.toml` из чата:

```toml
name = "Orange Pi Zero 3W"
arch = "arm64"
cpu = "H618"
kernel = "linux-6.12"
bootloader = "u-boot"
dtb = "sun50i-h618-orangepi-zero3.dtb"
rootfs = "ubuntu-base-26.04"
boot_partition = "fat32"
root_partition = "ext4"
```

Дополнительно планировались: `kernel.toml`, `uboot.toml`, `packages.list`.

### Context (состояние сборки)

Один объект вместо 20 аргументов между этапами:

- `work_dir` — рабочая директория (`build/work`)
- `output_dir` — готовые образы (`output`)
- позже: cache, board, config

### Stage trait

```rust
pub trait Stage {
    fn name(&self) -> &'static str;
    fn execute(&self, ctx: &mut Context) -> Result<()>;
}
```

Первый реализованный stage: **PrepareStage** — создаёт `work_dir` и `output_dir`.

---

## Часть 4. Обучающий подход (Rust)

- Не прыгать сразу в `clap` — сначала базовые конструкции Rust в проекте.
- Каждый файл писать полностью, каждый commit понимать.
- `main.rs` не должен содержать бизнес-логику — только делегирование в CLI → BuildEngine.
- Объяснять: `struct`, `impl`, `PathBuf`, `Result`, trait objects (`Box<dyn Stage>`).

---

## Часть 5. Будущие компоненты Platinum

| Компонент | Назначение |
|-----------|------------|
| `platinum update` | Обновление apt + kernel + DTB + firmware (не просто `apt upgrade`) |
| `platinum-config` | Конфигурация системы |
| `platinum-agent` | Собственный агент на устройстве |
| `platinum-firstboot` | Первый запуск |

---

## Текущий статус репозитория (на момент переноса контекста)

**Уже есть:**

- Rust workspace в `platinum/`
- `crates/build/src/context.rs` — struct Context с комментариями
- `crates/pipeline/` — Pipeline, Stage trait, PrepareStage
- `crates/cli/` — заглушка `Platinum Build System`

**Ещё заглушки (placeholder):**

- `board`, `rootfs`, `image`, `downloader` — boilerplate `add()`

**Ещё не создано:**

- `boards/orangepi-zero3w/`
- `BuildEngine`
- CLI с командами `build`, `version`
- downloader Ubuntu Base 26.04
- сборка kernel/bootloader/image

---

## Ссылки

- [Ubuntu Base 26.04 arm64](https://cdimage.ubuntu.com/ubuntu-base/releases/26.04/release/)
- [Ubuntu release cycle](https://ubuntu.com/about/release-cycle)
- Исходный чат: https://chatgpt.com/share/6a63b691-a298-83eb-b45c-293cd362882c
