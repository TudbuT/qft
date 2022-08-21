use std::{env, process, thread};

use iui::{controls::*, *};

#[derive(Clone)]
struct RefMut<T> {
    to: *mut T,
}

impl<T> RefMut<T> {
    pub fn new(to: &mut T) -> RefMut<T> {
        RefMut { to: to as *mut T }
    }

    pub fn get(&self) -> &mut T {
        unsafe { self.to.as_mut().unwrap() }
    }
}

unsafe impl<T> Sync for RefMut<T> {}

unsafe impl<T> Send for RefMut<T> {}

#[derive(Clone)]
struct Ref<T> {
    to: *const T,
}

impl<T> Ref<T> {
    pub fn new(to: &T) -> Ref<T> {
        Ref { to: to as *const T }
    }

    pub fn get(&self) -> &T {
        unsafe { self.to.as_ref().unwrap() }
    }
}

unsafe impl<T> Sync for Ref<T> {}

unsafe impl<T> Send for Ref<T> {}

fn wrap<T: Into<Control>>(ui: &UI, s: &str, c: T) -> Group {
    let mut g = Group::new(ui, s);
    g.set_child(ui, c);
    return g;
}

pub fn gui() -> Result<(), iui::UIError> {
    let ui: UI = UI::init()?;

    let mut window = Window::new(&ui, "QFT", 500, 600, WindowType::NoMenubar);
    let mut vbox = VerticalBox::new(&ui);
    vbox.set_padded(&ui, false);

    let mut mode = Combobox::new(&ui);
    let modeb = Ref::new(&mode);
    mode.append(&ui, "Receive");
    mode.append(&ui, "Send");
    mode.set_selected(&ui, 1);
    let mdb = Ref::new(&mode);
    vbox.append(&ui, wrap(&ui, "Mode: ", mode), LayoutStrategy::Compact);

    let mut helper_data = HorizontalBox::new(&ui);
    helper_data.set_padded(&ui, true);
    let mut helper = Entry::new(&ui);
    let helperb = Ref::new(&helper);
    helper.set_value(&ui, "tudbut.de:4277");
    helper_data.append(&ui, helper, LayoutStrategy::Stretchy);
    let mut phrase = Entry::new(&ui);
    let phraseb = Ref::new(&phrase);
    phrase.set_value(
        &ui,
        format!("my-cool-phrase-{}", rand::random::<u16>()).as_str(),
    );
    helper_data.append(&ui, phrase, LayoutStrategy::Stretchy);
    vbox.append(
        &ui,
        wrap(&ui, "Shared Phrase: ", helper_data),
        LayoutStrategy::Compact,
    );

    let mut path = env::current_exe()
        .unwrap()
        .into_os_string()
        .into_string()
        .unwrap();
    let pathb = Ref::new(&path);
    let mut select_path = Button::new(&ui, format!("Select a file ({})", path).as_str());
    let bb = RefMut::new(&mut select_path);
    let pb = RefMut::new(&mut path);
    let wb = Ref::new(&window);
    let uib = Ref::new(&ui);
    select_path.on_clicked(&ui, move |_| {
        let r = if mdb.get().selected(uib.get()) == 0 {
            wb.get()
                .save_file(uib.get())
                .map(|x| x.into_os_string().into_string().unwrap())
        } else {
            wb.get()
                .open_file(uib.get())
                .map(|x| x.into_os_string().into_string().unwrap())
        };
        match r {
            Some(path) => {
                bb.get()
                    .set_text(uib.get(), format!("Select a file ({})", &path).as_str());
                *pb.get() = path;
            }
            None => (),
        }
    });
    vbox.append(
        &ui,
        wrap(&ui, "Path: ", select_path),
        LayoutStrategy::Compact,
    );

    let mut speed = VerticalBox::new(&ui);
    let mut speed_slider = Slider::new(&ui, 100, 10_000);
    let speedb = Ref::new(&speed_slider);
    let mut speed_box = Entry::new(&ui);
    speed_slider.set_value(&ui, 256);
    speed_box.set_value(&ui, "256");
    // We know that ui.main() will wait until the UI is dead, so these are safe.
    let sb = RefMut::new(&mut speed_slider);
    let bb = RefMut::new(&mut speed_box);
    let uib = Ref::new(&ui);
    let uib1 = uib.clone();
    speed_box.on_changed(&ui, move |val| {
        sb.get().set_value(
            uib.get(),
            u16::from_str_radix(val.as_str(), 10).unwrap_or(256) as i32,
        );
    });
    speed_slider.on_changed(&ui, move |val| {
        bb.get().set_value(uib1.get(), val.to_string().as_str());
    });
    speed.set_padded(&ui, true);
    speed.append(&ui, speed_slider, LayoutStrategy::Compact);
    speed.append(&ui, speed_box, LayoutStrategy::Compact);
    vbox.append(
        &ui,
        wrap(
            &ui,
            "Bitrate: (lower = more reliable, higher = faster)",
            speed,
        ),
        LayoutStrategy::Compact,
    );

    let mut skip = Entry::new(&ui);
    let skipb = Ref::new(&skip);
    skip.set_value(&ui, "0");
    vbox.append(
        &ui,
        wrap(&ui, "Resume from: ", skip),
        LayoutStrategy::Compact,
    );

    let mut bar = ProgressBar::new();
    let barb = RefMut::new(&mut bar);
    bar.set_value(&ui, ProgressBarValue::Determinate(0));
    vbox.append(&ui, wrap(&ui, "Progress: ", bar), LayoutStrategy::Compact);

    let mut send_button = Button::new(&ui, "Start");
    let bb = RefMut::new(&mut send_button);
    let uib = Ref::new(&ui);
    send_button.on_clicked(&ui, move |b| {
        b.disable(uib.get());
        let bb = bb.clone();
        let rargs = env::args().collect::<Vec<String>>();
        let mut args = vec![rargs.get(0).unwrap().clone()];
        let a = String::from(match modeb.get().selected(uib.get()) {
            0 => "receiver",
            1 => "sender",
            _ => "version",
        });
        args.push(a);
        let a = helperb.get().value(uib.get());
        args.push(a);
        let a = phraseb.get().value(uib.get());
        args.push(a);
        let a = pathb.get().clone();
        args.push(a);
        let a = speedb.get().value(uib.get()).to_string();
        args.push(a);
        let a = skipb.get().value(uib.get());
        args.push(a);
        println!("{:?}", args);
        match modeb.get().selected(uib.get()) {
            0 => {
                barb.get()
                    .set_value(uib.get(), ProgressBarValue::Indeterminate);
                let uib1 = Ref::new(uib.get());
                let barb1 = RefMut::new(barb.get());

                thread::spawn(move || {
                    crate::receiver(&args);
                    let uib = uib1.clone();
                    let barb = barb1.clone();
                    uib1.get().queue_main(move || {
                        barb
                            .get()
                            .set_value(uib.get(), ProgressBarValue::Determinate(100));
                        bb.get().enable(uib.get());
                    });
                });
            }
            1 => {
                barb.get()
                    .set_value(uib.get(), ProgressBarValue::Indeterminate);
                let uib = uib.clone();
                let barb = barb.clone();

                thread::spawn(move || {
                    let mut last_percentage = 0;
                    let lpb = RefMut::new(&mut last_percentage);
                    let uib1 = uib.clone();
                    let barb1 = barb.clone();
                    crate::sender(&args, move |f| {
                        let lpb1 = lpb.clone();
                        let uib = uib1.clone();
                        let barb = barb1.clone();
                        uib1.get().queue_main(move || {
                            let percentage = (f * 100 as f32) as u32;
                            if percentage != *lpb1.get() {
                                barb.get().set_value(uib.get(), ProgressBarValue::Determinate(percentage));
                                *lpb1.get() = percentage;
                            }
                        })
                    });
                    let uib1 = uib.clone();
                    let barb1 = barb.clone();
                    uib.get().queue_main(move || {
                        barb1
                            .get()
                            .set_value(uib1.get(), ProgressBarValue::Determinate(100));
                        bb.get().enable(uib1.get());
                    });
                });
            }
            _ => panic!("invalid mode"),
        }
        println!("Running.");
    });
    vbox.append(
        &ui,
        wrap(&ui, "Start: ", send_button),
        LayoutStrategy::Compact,
    );

    let uib = Ref::new(&ui);
    window.on_closing(&ui, move |_| {
        let mut quit_window =
            Window::new(uib.get(), "Really quit?", 300, 100, WindowType::NoMenubar);
        let mut vbox = VerticalBox::new(uib.get());

        let label = Label::new(uib.get(), "Do you really want to quit?");
        vbox.append(uib.get(), label, LayoutStrategy::Compact);

        vbox.append(uib.get(), Spacer::new(uib.get()), LayoutStrategy::Stretchy);

        let mut hbox = HorizontalBox::new(uib.get());
        hbox.set_padded(uib.get(), true);
        let mut button1 = Button::new(uib.get(), "Quit");
        let uib1 = uib.clone();
        button1.on_clicked(uib.get(), move |_| {
            uib1.get().quit();
            process::exit(0);
        });
        let mut button2 = Button::new(uib.get(), "Cancel");
        let uib1 = uib.clone();
        // We know button2 won't be destroyed until quit_window is dead, so these are fine
        let qwb = RefMut::new(&mut quit_window);
        button2.on_clicked(uib.get(), move |_| quit_window.hide(uib1.get()));
        let quit_window = qwb.get();
        hbox.append(uib.get(), button1, LayoutStrategy::Stretchy);
        hbox.append(uib.get(), button2, LayoutStrategy::Stretchy);
        vbox.append(uib.get(), hbox, LayoutStrategy::Compact);

        quit_window.set_child(uib.get(), vbox);
        let uib1 = uib.clone();
        quit_window.on_closing(uib.get(), move |w| w.hide(uib1.get()));
        quit_window.show(uib.get());
    });

    window.set_child(&ui, vbox);
    window.show(&ui);

    /*si.on_should_quit(move || {
        let quit_window = Window::new(&ui, "Really quit?", 300, 100, WindowType::NoMenubar);
        let label = Label::new(&ui, "Do you really want to quit? Data is currently being transferred.");
        ui.quit();
    });*/

    ui.event_loop().run(&ui);
    println!("GUI done");

    return Ok(());
}
