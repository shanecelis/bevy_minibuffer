use bevy::prelude::*;
use bevy_minibuffer::prelude::*;

#[derive(Deref, DerefMut)]
// Found in bevy's gamepad.rs.
struct TestContext {
    pub app: App,
}

impl TestContext {
    fn press_key(&mut self, key: KeyCode) {
        self.app
            .world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
    }

    fn clear_just_pressed(&mut self, key: KeyCode) {
        self.app
            .world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear_just_pressed(key);
    }

    fn release_key(&mut self, key: KeyCode) {
        self.app
            .world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(key);
    }

    fn state<S: States>(&self) -> &S {
        self.app.world().resource::<State<S>>().get()
    }
}

fn new_app() -> TestContext {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        bevy::input::InputPlugin,
        bevy::asset::AssetPlugin::default(),
        // XXX: TextPlugin causes issues.
        // bevy::text::TextPlugin,
        // ImagePlugin::default(),
        // bevy::sprite::SpritePlugin,
        bevy::state::app::StatesPlugin,
        // MinibufferPlugins,
    ));
    // app.init_resource::<ButtonInput<KeyCode>>();
    app.add_event::<bevy::window::RequestRedraw>();
    // app.add_systems(PostUpdate, read);
    TestContext { app }
}

mod two_commands {
    use super::*;

    /// Ask the user for their name. Say hello.
    fn ask_name(mut minibuffer: Minibuffer) {
        minibuffer
            .prompt::<TextField>("What's your first name? ")
            .observe(
                |mut trigger: Trigger<Submit<String>>, mut minibuffer: Minibuffer| {
                    let first_name = trigger.event_mut().take_result().unwrap();
                    minibuffer
                        .prompt::<TextField>("What's your last name? ")
                        .observe(
                            move |mut trigger: Trigger<Submit<String>>,
                                  mut minibuffer: Minibuffer| {
                                let last_name = trigger.event_mut().take_result().unwrap();
                                minibuffer.message(format!("Hello, {first_name} {last_name}!"));
                            },
                        );
                },
            );
    }

    // Ask the user for their age.
    fn ask_age(mut minibuffer: Minibuffer) {
        minibuffer
            .prompt::<Number<u8>>("What's your age? ")
            .observe(
                |mut trigger: Trigger<Submit<u8>>, mut minibuffer: Minibuffer| {
                    let age = trigger.event_mut().take_result().unwrap();
                    minibuffer.message(format!("You are {age} years old."));
                },
            );
    }

    fn new_app() -> TestContext {
        let mut app = super::new_app();
        app.add_acts((
            Act::new(ask_name).named("ask_name").bind(keyseq!(N)),
            Act::new(ask_age).named("ask_age").bind(keyseq!(A)),
            // Add a basic act but just one of them.
            BasicActs::default().remove("run_act").unwrap(),
        ));
        // .add_systems(Startup, |mut minibuffer: Minibuffer| {
        //     minibuffer.message("Hit 'N' for ask_name. Hit 'A' for ask_age.");
        //     minibuffer.set_visible(true);
        // });
        app
    }

    #[test]
    fn run_ask_name() {
        let mut app = new_app();
        // assert_eq!(*app.state::<MinibufferState>(), MinibufferState::Inactive);
        // app.press_key(KeyCode::KeyN);
        app.update();
        // assert_ne!(*app.state::<MinibufferState>(), MinibufferState::Inactive);
    }
}
