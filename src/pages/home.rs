use leptos::{component, view, IntoView};

#[component]
pub fn Home() -> impl IntoView {
    view! {
        <div class="row">
            <div class="cell" style:height=format!("calc(126px + (2.2em * {}))", 2)>
                <div class="celltitle">
                    Torid Viva-concinak
                </div>
                <hr/>
                    <p style="text-align: center;">+16.5% Heat</p>
                    <p style="text-align: center; margin-block-start: 0; margin-block-end: 0;">+16.5% Heat</p>
                <div class="cellbuttondiv">
                    <button class="cellbutton">Edit</button>
                    <button class="cellbutton" style="background-color: #ff4444;">Delete</button>
                </div>
            </div>
            <div class="cell" style:height=format!("calc(126px + (2.2em * {}))", 4)>
                <div class="celltitle">
                   Skana scipha
                </div>
                <hr/>
                    <p style="text-align: center;">+16.5% Heat</p>
                    <p style="text-align: center;">+22.7% Slash</p>
                    <p style="text-align: center;">-3.7% Fire Rate / Attack Speed</p>
                    <p style="text-align: center;">-3.7% Fire Rate / Attack Speed</p>
                <div class="cellbuttondiv">
                    <button class="cellbutton">Edit</button>
                    <button class="cellbutton" style="background-color: #ff4444;">Delete</button>
                </div>
            </div>
        </div>
    }
}
