# platinum-armbian-bsp

[← Модули](README.md)

## Назначение

Адаптер pinned Armbian Build. Platinum не копирует shell-логику Armbian, а
запускает официальный `compile.sh` на проверенном checkout.

## Публичный API

| Тип | Роль |
| --- | --- |
| `ArmbianCheckout` | Clone/fetch/detach, проверка origin + HEAD |
| `ArmbianBspRunner` | Targets `kernel` / `uboot` с повторной проверкой HEAD |
| `BspInventory` | Поиск собранных `.deb` |
| `KernelArtifacts` | Результат поиска kernel/DTB |
| Ошибки | `ArmbianBspError`, `InventoryError` |

## Правила безопасности

- Revision — только 40-символьный hex SHA.
- Неверный `origin` в cache отклоняется.
- HEAD должен совпадать с pin до compile.
- Armbian rootfs/image не является Platinum userspace.

## Зависимости

`platinum-board`, `thiserror`, `tracing`
