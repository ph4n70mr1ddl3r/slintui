use slint::ComponentHandle;
use slint_poker::MainWindow;

fn main() {
    let main_window = MainWindow::new().unwrap();
    main_window.run().unwrap();
}
