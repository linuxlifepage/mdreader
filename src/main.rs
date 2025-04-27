use eframe::egui::{Color32, FontFamily, FontId, TextStyle, Visuals, RichText, FontData, FontDefinitions};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use std::fs;
use pulldown_cmark::{Parser, Event, Tag, HeadingLevel};

struct MdReader {
    current_dir: PathBuf,
    root_dir: PathBuf,
    categories: Vec<Category>,
    selected_file: Option<PathBuf>,
    edit_mode: bool,
    file_content: String,
    sidebar_width: f32,
    new_category_name: String,
    new_file_name: String,
    show_new_category_dialog: bool,
    show_new_file_dialog: bool,
    dark_mode: bool,
}

struct Category {
    name: String,
    path: PathBuf,
    files: Vec<FileEntry>,
    subcategories: Vec<Category>,
    is_expanded: bool,
}

struct FileEntry {
    name: String,
    path: PathBuf,
}

impl MdReader {
    fn new() -> Self {
        let root_dir = std::env::current_dir().unwrap();
        let mut app = Self {
            current_dir: root_dir.clone(),
            root_dir,
            categories: Vec::new(),
            selected_file: None,
            edit_mode: false,
            file_content: String::new(),
            sidebar_width: 300.0,
            new_category_name: String::new(),
            new_file_name: String::new(),
            show_new_category_dialog: false,
            show_new_file_dialog: false,
            dark_mode: true,
        };
        app.scan_directory();
        app
    }

    fn scan_directory(&mut self) {
        self.categories.clear();
        
        for entry in WalkDir::new(&self.root_dir)
            .min_depth(1)
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_dir() {
                let mut category = Category {
                    name: entry.file_name().to_string_lossy().to_string(),
                    path: entry.path().to_path_buf(),
                    files: Vec::new(),
                    subcategories: Vec::new(),
                    is_expanded: false,
                };
                
                self.scan_category_recursively(&mut category);
                self.categories.push(category);
            }
        }

        // Восстанавливаем состояние развернутости для текущей директории
        let current_path = self.current_dir.clone();
        self.expand_path_to(&current_path);
    }

    fn expand_path_to(&mut self, target_path: &Path) {
        fn expand_in_category(category: &mut Category, target_path: &Path) -> bool {
            if target_path.starts_with(&category.path) {
                category.is_expanded = true;
                for subcategory in &mut category.subcategories {
                    if expand_in_category(subcategory, target_path) {
                        return true;
                    }
                }
                return true;
            }
            false
        }

        for category in &mut self.categories {
            expand_in_category(category, target_path);
        }
    }

    fn scan_category_recursively(&self, category: &mut Category) {
        self.scan_files_in_category(category);
        
        for entry in WalkDir::new(&category.path)
            .min_depth(1)
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_dir() {
                let mut subcategory = Category {
                    name: entry.file_name().to_string_lossy().to_string(),
                    path: entry.path().to_path_buf(),
                    files: Vec::new(),
                    subcategories: Vec::new(),
                    is_expanded: false,
                };
                
                self.scan_category_recursively(&mut subcategory);
                category.subcategories.push(subcategory);
            }
        }
    }

    fn scan_files_in_category(&self, category: &mut Category) {
        for entry in WalkDir::new(&category.path)
            .min_depth(1)
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() && entry.path().extension().map_or(false, |ext| ext == "md") {
                if let Ok(content) = fs::read_to_string(entry.path()) {
                    let title = content.lines()
                        .next()
                        .unwrap_or("")
                        .trim_start_matches(|c| c == '#' || c == ' ')
                        .chars()
                        .take(35)
                        .collect::<String>();

                    category.files.push(FileEntry {
                        name: title,
                        path: entry.path().to_path_buf(),
                    });
                }
            }
        }
    }

    fn save_file(&self) -> Result<(), std::io::Error> {
        if let Some(path) = &self.selected_file {
            fs::write(path, &self.file_content)
        } else {
            Ok(())
        }
    }

    fn load_file(&mut self, path: &Path) {
        if let Ok(content) = fs::read_to_string(path) {
            self.file_content = content;
            self.selected_file = Some(path.to_path_buf());
        }
    }

    fn create_category(&mut self) {
        if !self.new_category_name.is_empty() {
            let new_path = self.current_dir.join(&self.new_category_name);
            if !new_path.exists() {
                if let Ok(_) = fs::create_dir(&new_path) {
                    // Сохраняем состояние развернутости категорий
                    let expanded_states: Vec<(PathBuf, bool)> = self.categories
                        .iter()
                        .map(|c| (c.path.clone(), c.is_expanded))
                        .collect();
                    
                    self.new_category_name.clear();
                    self.show_new_category_dialog = false;
                    self.scan_directory();
                    
                    // Восстанавливаем состояние развернутости
                    for category in &mut self.categories {
                        if let Some(state) = expanded_states.iter().find(|(path, _)| path == &category.path) {
                            category.is_expanded = state.1;
                        }
                    }
                }
            }
        }
    }

    fn create_file(&mut self) {
        if !self.new_file_name.is_empty() {
            let file_name = if self.new_file_name.ends_with(".md") {
                self.new_file_name.clone()
            } else {
                format!("{}.md", self.new_file_name)
            };
            
            let file_path = self.current_dir.join(&file_name);
            if !file_path.exists() {
                if let Ok(_) = fs::write(&file_path, format!("# {}\n", self.new_file_name)) {
                    // Сохраняем состояние развернутости категорий
                    let expanded_states: Vec<(PathBuf, bool)> = self.categories
                        .iter()
                        .map(|c| (c.path.clone(), c.is_expanded))
                        .collect();
                    
                    self.new_file_name.clear();
                    self.show_new_file_dialog = false;
                    self.scan_directory();
                    
                    // Восстанавливаем состояние развернутости
                    for category in &mut self.categories {
                        if let Some(state) = expanded_states.iter().find(|(path, _)| path == &category.path) {
                            category.is_expanded = state.1;
                        }
                    }
                    
                    self.load_file(&file_path);
                }
            }
        }
    }

    fn render_category(&mut self, ui: &mut egui::Ui, category: &mut Category) {
        let (button_color, category_color) = if self.dark_mode {
            (
                Color32::from_rgb(27, 33, 56),
                Color32::from_rgb(71, 130, 218)  // Синий цвет для категорий в темной теме
            )
        } else {
            (
                Color32::from_rgb(255, 255, 255),
                Color32::from_rgb(230, 240, 255)  // Светло-синий цвет для категорий в светлой теме
            )
        };
        
        let text_color = if self.dark_mode {
            Color32::from_rgb(220, 220, 240)
        } else {
            Color32::from_rgb(33, 33, 43)
        };
        
        // Рендерим категорию с особым стилем
        let response = ui.add(
            egui::Button::new(
                RichText::new(format!("📁 {}", category.name))
                    .color(text_color)
                    .size(16.0)  // Было 16.0
                    .strong()    // Делаем текст категорий жирным
            )
            .fill(category_color)
            .rounding(10.0)     // Было 8.0
            .min_size(egui::vec2(ui.available_width(), 16.0))  // Минимальная высота кнопки
        );
        
        if response.clicked() {
            category.is_expanded = !category.is_expanded;
            self.current_dir = category.path.clone();
        }
        
        if category.is_expanded {
            ui.indent("category_indent", |ui| {
                // Добавляем отступ для файлов
                ui.add_space(5.0);
                
                for file in &category.files {
                    let file_response = ui.add(
                        egui::Button::new(
                            RichText::new(format!("📄 {}", file.name))
                                .color(text_color)
                                .size(14.0)  // Оставляем файлы немного меньше категорий
                        )
                        .fill(button_color)
                        .rounding(8.0)
                        .min_size(egui::vec2(ui.available_width(), 28.0))  // Чуть меньше высота для файлов
                    );
                    
                    if file_response.clicked() {
                        self.load_file(&file.path);
                    }
                }
                
                ui.add_space(5.0);
                
                for subcategory in &mut category.subcategories {
                    self.render_category(ui, subcategory);
                }
            });
        }
    }

    fn render_markdown(&self, ui: &mut egui::Ui, content: &str) {
        let parser = Parser::new(content);
        let mut current_text = String::new();
        let mut in_code_block = false;
        let mut in_list = false;
        let mut current_heading_level = HeadingLevel::H1;
        
        for event in parser {
            match event {
                Event::Start(Tag::Heading(level, _, _)) => {
                    if !current_text.is_empty() {
                        ui.label(&current_text);
                        current_text.clear();
                    }
                    current_heading_level = level;
                }
                Event::End(Tag::Heading(..)) => {
                    let font_size = match current_heading_level {
                        HeadingLevel::H1 => 24.0,
                        HeadingLevel::H2 => 20.0,
                        HeadingLevel::H3 => 18.0,
                        HeadingLevel::H4 => 16.0,
                        HeadingLevel::H5 => 14.0,
                        HeadingLevel::H6 => 12.0,
                    };
                    ui.heading(RichText::new(&current_text)
                        .size(font_size)
                        .color(Color32::from_rgb(200, 200, 200)));
                    current_text.clear();
                }
                Event::Start(Tag::CodeBlock(_)) => {
                    in_code_block = true;
                }
                Event::End(Tag::CodeBlock(_)) => {
                    in_code_block = false;
                    ui.add(egui::Label::new(RichText::new(&current_text)
                        .monospace()
                        .color(Color32::from_rgb(150, 150, 150))));
                    current_text.clear();
                }
                Event::Start(Tag::List(_)) => {
                    in_list = true;
                }
                Event::End(Tag::List(_)) => {
                    in_list = false;
                }
                Event::Text(text) => {
                    if in_code_block {
                        current_text.push_str(&text);
                    } else if in_list {
                        ui.label(format!("• {}", text));
                    } else {
                        current_text.push_str(&text);
                    }
                }
                Event::SoftBreak => {
                    if !in_code_block {
                        current_text.push('\n');
                    }
                }
                Event::HardBreak => {
                    if !in_code_block {
                        current_text.push('\n');
                    }
                }
                _ => {}
            }
        }
        
        if !current_text.is_empty() {
            ui.label(&current_text);
        }
    }

    fn toggle_theme(&mut self) {
        self.dark_mode = !self.dark_mode;
    }

    fn setup_fonts(ctx: &egui::Context) {
        let mut fonts = FontDefinitions::default();
        
        // Добавляем Roboto как основной шрифт
        fonts.font_data.insert(
            "roboto".to_owned(),
            FontData::from_static(include_bytes!("../assets/Roboto-Regular.ttf")),
        );
        
        // Настраиваем семейства шрифтов
        fonts.families.get_mut(&FontFamily::Proportional).unwrap()
            .insert(0, "roboto".to_owned());
        
        ctx.set_fonts(fonts);
    }
}

impl eframe::App for MdReader {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut style = (*ctx.style()).clone();
        
        // Корректируем размеры шрифтов
        style.text_styles.insert(
            TextStyle::Heading,
            FontId::new(28.0, FontFamily::Proportional), // Уменьшили с 30.0
        );
        style.text_styles.insert(
            TextStyle::Body,
            FontId::new(17.0, FontFamily::Proportional), // Уменьшили с 18.0
        );
        style.text_styles.insert(
            TextStyle::Button,
            FontId::new(17.0, FontFamily::Proportional), // Уменьшили с 18.0
        );

        // Увеличиваем отступы
        style.spacing.item_spacing = egui::vec2(10.0, 10.0);
        style.spacing.button_padding = egui::vec2(15.0, 10.0); // Увеличили padding для кнопок
        
        let mut visuals = if self.dark_mode {
            Visuals::dark()
        } else {
            Visuals::light()
        };
        
        // Новая цветовая палитра в стиле Berry Dashboard
        if self.dark_mode {
            // Основные цвета темной темы
            let bg_dark = Color32::from_rgb(17, 23, 43);     // Темно-синий фон
            let surface = Color32::from_rgb(27, 33, 56);     // Поверхность карточек
            let accent = Color32::from_rgb(145, 85, 253);    // Акцентный фиолетовый
            
            visuals.widgets.noninteractive.bg_fill = bg_dark;
            visuals.widgets.inactive.bg_fill = surface;
            visuals.widgets.hovered.bg_fill = Color32::from_rgb(37, 43, 66);
            visuals.widgets.active.bg_fill = Color32::from_rgb(47, 53, 76);
            visuals.widgets.inactive.fg_stroke.color = Color32::from_rgb(220, 220, 240);
            visuals.widgets.hovered.fg_stroke.color = Color32::WHITE;
            visuals.widgets.active.fg_stroke.color = Color32::WHITE;
            
            visuals.panel_fill = bg_dark;
            visuals.window_fill = surface;
            visuals.window_stroke.color = accent;
            visuals.window_stroke.width = 1.0;
            
            // Настройка кнопок
            visuals.widgets.inactive.rounding = egui::Rounding::same(8.0);
            visuals.widgets.hovered.rounding = egui::Rounding::same(8.0);
            visuals.widgets.active.rounding = egui::Rounding::same(8.0);
            visuals.window_rounding = egui::Rounding::same(12.0);
            
            // Тени для элементов
            visuals.popup_shadow = egui::epaint::Shadow {
                extrusion: 8.0,
                color: Color32::from_rgba_premultiplied(0, 0, 0, 96),
            };
            
            // Настройка выделения
            visuals.selection.bg_fill = accent;
            visuals.selection.stroke.color = Color32::WHITE;
        } else {
            // Светлая тема (можно оставить как есть или тоже настроить)
            visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(250, 250, 255);
            visuals.widgets.inactive.bg_fill = Color32::from_rgb(255, 255, 255);
            visuals.widgets.hovered.bg_fill = Color32::from_rgb(245, 245, 250);
            visuals.widgets.active.bg_fill = Color32::from_rgb(238, 238, 245);
            visuals.widgets.inactive.fg_stroke.color = Color32::from_rgb(33, 33, 43);
            visuals.widgets.hovered.fg_stroke.color = Color32::from_rgb(33, 33, 43);
            visuals.widgets.active.fg_stroke.color = Color32::from_rgb(33, 33, 43);
            visuals.panel_fill = Color32::from_rgb(250, 250, 255);
            visuals.window_fill = Color32::from_rgb(255, 255, 255);
            visuals.window_stroke.color = Color32::from_rgb(224, 224, 234);
        }
        
        ctx.set_style(style);
        ctx.set_visuals(visuals);

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.add_space(5.0); // Добавляем отступ сверху
            ui.horizontal(|ui| {
                ui.heading(RichText::new("MD Reader").size(28.0));  // Уменьшили с 30.0
                ui.add_space(40.0);
                
                let button_text = RichText::new("Создать категорию").size(17.0);
                if ui.add(
                    egui::Button::new(button_text)
                        .rounding(10.0)
                        .min_size(egui::vec2(140.0, 35.0)) // Увеличили размер кнопок в топ меню
                ).clicked() {
                    self.show_new_category_dialog = true;
                }
                
                let button_text = RichText::new("Создать заметку").size(17.0);
                if ui.add(
                    egui::Button::new(button_text)
                        .rounding(10.0)
                        .min_size(egui::vec2(120.0, 35.0))
                ).clicked() {
                    self.show_new_file_dialog = true;
                }
                
                let mode_text = if self.edit_mode { "Режим чтения" } else { "Режим редактирования" };
                let button_text = RichText::new(mode_text).size(17.0);
                if ui.add(
                    egui::Button::new(button_text)
                        .rounding(10.0)
                        .min_size(egui::vec2(160.0, 35.0))
                ).clicked() {
                    self.edit_mode = !self.edit_mode;
                }
                
                let theme_text = if self.dark_mode { "🌞 Светлая тема" } else { "🌙 Темная тема" };
                let button_text = RichText::new(theme_text).size(17.0);
                if ui.add(
                    egui::Button::new(button_text)
                        .rounding(10.0)
                        .min_size(egui::vec2(140.0, 35.0))
                ).clicked() {
                    self.toggle_theme();
                }
            });
            ui.add_space(5.0); // Добавляем отступ снизу
        });

        if self.show_new_category_dialog {
            let mut should_create = false;
            let mut dialog_open = self.show_new_category_dialog;
            
            egui::Window::new("Новая категория")
                .open(&mut dialog_open)
                .show(ctx, |ui| {
                    ui.label("Введите имя категории:");
                    let text_edit_response = ui.text_edit_singleline(&mut self.new_category_name);
                    let button_response = ui.button("Создать");
                    
                    if button_response.clicked() || (text_edit_response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) {
                        should_create = true;
                    }
                });

            if should_create {
                self.create_category();
                dialog_open = false;
            }
            self.show_new_category_dialog = dialog_open;
        }

        if self.show_new_file_dialog {
            let mut should_create = false;
            let mut dialog_open = self.show_new_file_dialog;
            
            egui::Window::new("Новая заметка")
                .open(&mut dialog_open)
                .show(ctx, |ui| {
                    ui.label("Введите название заметки:");
                    let text_edit_response = ui.text_edit_singleline(&mut self.new_file_name);
                    let button_response = ui.button("Создать");
                    
                    if button_response.clicked() || (text_edit_response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) {
                        should_create = true;
                    }
                });

            if should_create {
                self.create_file();
                dialog_open = false;
            }
            self.show_new_file_dialog = dialog_open;
        }

        egui::SidePanel::left("sidebar")
            .resizable(true)
            .min_width(200.0)
            .max_width(600.0)
            .default_width(self.sidebar_width)
            .show(ctx, |ui| {
                // Add a vertical ScrollArea
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let categories = std::mem::take(&mut self.categories);
                    for mut category in categories {
                        self.render_category(ui, &mut category);
                        self.categories.push(category);
                    }
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(10.0); // Уменьшаем отступ сверху
            if let Some(_path) = &self.selected_file {
                let content_width = ui.available_width() - 20.0; // Уменьшаем боковые отступы
                
                if self.edit_mode {
                    let response = ui.add(
                        egui::TextEdit::multiline(&mut self.file_content)
                            .desired_width(content_width)
                            .desired_rows(30)
                            .margin(egui::vec2(10.0, 10.0)) // Уменьшаем внутренние отступы
                    );
                    
                    if response.changed() {
                        if let Err(e) = self.save_file() {
                            eprintln!("Ошибка сохранения файла: {}", e);
                        }
                    }
                } else {
                    ui.add_space(5.0);
                    let mut text_ui = ui.child_ui(
                        egui::Rect::from_min_size(
                            ui.min_rect().min + egui::vec2(10.0, 0.0), // Уменьшаем отступ слева
                            egui::vec2(content_width, ui.available_height())
                        ),
                        *ui.layout()
                    );
                    self.render_markdown(&mut text_ui, &self.file_content);
                    ui.add_space(5.0);
                }
            }
            ui.add_space(10.0); // Уменьшаем отступ снизу
        });
    }
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_decorations(true)
            .with_transparent(false),
        ..Default::default()
    };
    
    eframe::run_native(
        "MD Reader",
        options,
        Box::new(|cc| {
            MdReader::setup_fonts(&cc.egui_ctx);
            Box::new(MdReader::new())
        }),
    )
} 