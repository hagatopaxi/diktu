use adw::prelude::*;

const APP_ID: &str = "fr.gwenael_leger.Parlotte";

fn main() -> gtk::glib::ExitCode {
    let app = adw::Application::builder().application_id(APP_ID).build();
    app.connect_activate(|app| {
        adw::ApplicationWindow::builder()
            .application(app)
            .title("Parlotte")
            .build()
            .present();
    });
    app.run()
}
