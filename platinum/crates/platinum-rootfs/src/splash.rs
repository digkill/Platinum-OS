//! Заставка при загрузке через Plymouth.
//!
//! Plymouth рисует картинку с раннего этапа загрузки до появления оболочки,
//! закрывая собой лог ядра. Работает поверх KMS, поэтому на платах без драйвера
//! дисплея заставки не будет — но там и оболочки нет.
//!
//! Тема ставится сборкой, а не берётся готовая: изображение — часть продукта, а
//! не системная настройка, и в пакете его быть не может.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use thiserror::Error;
use tracing::info;

use crate::chroot::ChrootSession;

/// Имя темы Plymouth.
const THEME: &str = "platinum";

/// Каталог тем Plymouth внутри rootfs.
const THEMES_DIRECTORY: &str = "usr/share/plymouth/themes";

/// Имя файла изображения внутри темы.
const LOGO_FILE: &str = "logo.png";

/// Ошибки установки заставки.
#[derive(Debug, Error)]
pub enum SplashError {
    /// Изображения нет по указанному пути.
    #[error("изображение заставки отсутствует: {path}")]
    MissingImage {
        /// Ожидавшийся файл.
        path: PathBuf,
    },
    /// Файл не удалось записать.
    #[error("не удалось записать `{path}`: {source}")]
    Write {
        /// Проблемный путь.
        path: PathBuf,
        /// Исходная ошибка файловой системы.
        #[source]
        source: io::Error,
    },
}

/// Параметры заставки.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplashSpec {
    /// Изображение заставки на хосте сборки.
    ///
    /// Приходит уже нужного размера: Plymouth в Ubuntu не масштабирует
    /// изображение из скрипта — `Image.Scale()` даёт пустой спрайт, проверено
    /// загрузкой. Картинка крупнее экрана будет обрезана, поэтому её сторона
    /// должна быть примерно вдвое меньше короткой стороны панели.
    pub image: PathBuf,
    /// Цвет фона в формате `#rrggbb`.
    pub background: String,
}

/// Ставит тему Plymouth с изображением продукта.
#[derive(Debug, Clone)]
pub struct SplashConfigurator {
    spec: SplashSpec,
}

impl SplashConfigurator {
    /// Создаёт конфигуратор заставки.
    pub fn new(spec: SplashSpec) -> Self {
        Self { spec }
    }

    /// Раскладывает тему в rootfs.
    pub fn install(&self, rootfs: &Path) -> Result<(), SplashError> {
        if !self.spec.image.is_file() {
            return Err(SplashError::MissingImage {
                path: self.spec.image.clone(),
            });
        }

        let theme = rootfs.join(THEMES_DIRECTORY).join(THEME);
        fs::create_dir_all(&theme).map_err(|source| SplashError::Write {
            path: theme.clone(),
            source,
        })?;

        fs::copy(&self.spec.image, theme.join(LOGO_FILE)).map_err(|source| SplashError::Write {
            path: theme.join(LOGO_FILE),
            source,
        })?;

        write(&theme.join(format!("{THEME}.plymouth")), &render_theme())?;
        write(
            &theme.join(format!("{THEME}.script")),
            &render_script(&self.spec.background),
        )?;

        info!(theme = THEME, "boot splash installed");

        Ok(())
    }

    /// Делает тему темой по умолчанию и пересобирает initramfs.
    ///
    /// Тема выбирается через `update-alternatives`: именно так её ищет хук
    /// initramfs — `update-alternatives --query default.plymouth`. Утилиты
    /// `plymouth-set-default-theme` в современной Ubuntu нет вовсе, и вызов
    /// завершался кодом 127.
    ///
    /// Пересборка initramfs обязательна: Plymouth копирует туда ту тему,
    /// которая назначена на момент сборки. Без неё заставка появилась бы лишь
    /// после монтирования корня, то есть под самый конец загрузки.
    pub fn activate(&self, session: &ChrootSession<'_>) -> Result<(), crate::ChrootError> {
        let theme_file = format!("/{THEMES_DIRECTORY}/{THEME}/{THEME}.plymouth");
        let link = format!("/{THEMES_DIRECTORY}/default.plymouth");

        // Приоритет выше пакетных тем, иначе выбор достался бы им.
        session.run(
            "update-alternatives",
            &["--install", &link, "default.plymouth", &theme_file, "200"],
        )?;
        session.run(
            "update-alternatives",
            &["--set", "default.plymouth", &theme_file],
        )?;

        // Демон вне initramfs читает тему отсюда.
        session.run(
            "sh",
            &[
                "-c",
                &format!("printf '[Daemon]\\nTheme={THEME}\\n' > /etc/plymouth/plymouthd.conf"),
            ],
        )?;

        session.run("update-initramfs", &["-u"])
    }
}

/// Записывает текстовый файл.
fn write(path: &Path, contents: &str) -> Result<(), SplashError> {
    fs::write(path, contents).map_err(|source| SplashError::Write {
        path: path.to_path_buf(),
        source,
    })
}

/// Формирует описание темы.
fn render_theme() -> String {
    format!(
        "# Platinum OS: файл создан сборкой, ручные правки будут перезаписаны.\n\
         [Plymouth Theme]\n\
         Name=Platinum OS\n\
         Description=Заставка загрузки Platinum OS\n\
         ModuleName=script\n\
         \n\
         [script]\n\
         ImageDir=/{THEMES_DIRECTORY}/{THEME}\n\
         ScriptFile=/{THEMES_DIRECTORY}/{THEME}/{THEME}.script\n"
    )
}

/// Формирует скрипт отрисовки.
///
/// Форма скрипта не выбрана, а измерена на живой машине 2026-08-07. Plymouth
/// не печатает причин, по которым спрайт остаётся пустым, поэтому каждое
/// свойство проверялось отдельной загрузкой:
///
/// - `Window.GetWidth()` на верхнем уровне возвращает 0: экрана в момент
///   разбора скрипта ещё нет. Поэтому положение считается в `refresh`, когда
///   окно уже существует;
/// - `Image.Scale()` не работает — спрайт получается пустой, экран чёрный.
///   Поэтому изображение должно приходить уже нужного размера, см.
///   [`SplashSpec::image`];
/// - функции и `Plymouth.SetRefreshFunction` работают.
///
/// Главное же условие лежит вне скрипта: при `console=ttyS*`/`ttyAMA*` в
/// командной строке ядра Plymouth принудительно включает текстовую тему
/// (`serial consoles detected, managing them with details forced`), и никакая
/// тема не покажется. Плата, которой нужна заставка, не должна объявлять
/// последовательную консоль.
fn render_script(background: &str) -> String {
    let (red, green, blue) = parse_color(background);

    format!(
        "# Platinum OS: файл создан сборкой, ручные правки будут перезаписаны.\n\
         Window.SetBackgroundTopColor({red:.3}, {green:.3}, {blue:.3});\n\
         Window.SetBackgroundBottomColor({red:.3}, {green:.3}, {blue:.3});\n\
         \n\
         logo.image = Image(\"{LOGO_FILE}\");\n\
         logo.sprite = Sprite(logo.image);\n\
         \n\
         # Положение считается на каждой перерисовке: на верхнем уровне размер\n\
         # окна ещё нулевой, и логотип уехал бы в угол.\n\
         fun refresh() {{\n\
         \x20   logo.sprite.SetX(Window.GetWidth() / 2 - logo.image.GetWidth() / 2);\n\
         \x20   logo.sprite.SetY(Window.GetHeight() / 2 - logo.image.GetHeight() / 2);\n\
         }}\n\
         \n\
         Plymouth.SetRefreshFunction(refresh);\n"
    )
}

/// Переводит `#rrggbb` в доли единицы, как их ждёт Plymouth.
///
/// Некорректное значение не отклоняется: цвет фона — деталь оформления, и
/// ронять из-за него сборку образа хуже, чем показать чёрный экран.
fn parse_color(value: &str) -> (f32, f32, f32) {
    let digits = value.trim_start_matches('#');
    if digits.len() != 6 {
        return (0.0, 0.0, 0.0);
    }

    let component =
        |from: usize| u8::from_str_radix(&digits[from..from + 2], 16).unwrap_or(0) as f32 / 255.0;

    (component(0), component(2), component(4))
}

#[cfg(test)]
mod tests {
    use super::{parse_color, render_script, render_theme};

    #[test]
    fn describes_a_script_theme() {
        let theme = render_theme();

        assert!(theme.contains("ModuleName=script\n"));
        assert!(theme.contains("ImageDir=/usr/share/plymouth/themes/platinum\n"));
    }

    /// Скрипт обязан держаться того, что проверено загрузкой.
    ///
    /// Проверено на живой машине 2026-08-07: масштабирование даёт пустой
    /// спрайт, а размер окна на верхнем уровне равен нулю. Обе ошибки Plymouth
    /// не печатает — экран просто остаётся чёрным.
    #[test]
    fn stays_within_constructs_plymouth_executes() {
        let script = render_script("#000000");

        assert!(script.contains("Sprite(logo.image)"));
        // Центровка обязана считаться в перерисовке: на верхнем уровне окно
        // ещё нулевого размера.
        assert!(script.contains("Plymouth.SetRefreshFunction(refresh)"));
        assert!(!script.contains(").Scale("));
    }

    #[test]
    fn converts_the_background_colour() {
        let (red, green, blue) = parse_color("#ff8000");

        assert!((red - 1.0).abs() < 0.01);
        assert!((green - 0.502).abs() < 0.01);
        assert!(blue.abs() < 0.01);
    }

    /// Испорченный цвет не должен ронять сборку образа.
    #[test]
    fn falls_back_on_a_malformed_colour() {
        assert_eq!(parse_color("не цвет"), (0.0, 0.0, 0.0));
    }
}
