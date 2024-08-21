use cursive::views::{
    FixedLayout,
    HideableView,
    LinearLayout,
    Panel,
    ProgressBar,
    TextView,
};
use cursive::view::Nameable;
use cursive::utils::Counter;
use cursive::Rect;
use cursive::align::HAlign;
use super::{PANEL_WIDTH, TopView};
use libamdgpu_top::stat::GpuActivity;

const TITLE: &str = "Activity";
const ACTIVITY_LABEL_WIDTH: usize = 7;

#[derive(Clone, Debug)]
pub struct ActivityView {
    gfx_counter: Counter,
    umc_counter: Counter,
    media_counter: Counter,
    index: usize,
}

impl ActivityView {
    pub fn new(index: usize) -> Self {
        Self {
            gfx_counter: Counter::new(0),
            umc_counter: Counter::new(0),
            media_counter: Counter::new(0),
            index,
        }
    }

    pub fn view(&self, activity: &GpuActivity) -> TopView {
        const BAR_WIDTH: usize = PANEL_WIDTH / 3 - ACTIVITY_LABEL_WIDTH;

        let title = TITLE.to_string();
        let label = |value: usize, (_min, _max): (usize, usize)| -> String {
            let val = format!("{:>3} %", value);
            format!("[{val:^width$}]", width = BAR_WIDTH - 2)
        };
        let mut sub_layout = LinearLayout::horizontal();

        for (flag, counter, name) in [
            (activity.gfx.is_some(), &self.gfx_counter, "GFX"),
            (activity.umc.is_some(), &self.umc_counter, "UMC"),
            (activity.media.is_some(), &self.media_counter, "Media"),
        ] {
            if !flag { continue; }

            sub_layout.add_child(
                FixedLayout::new()
                    .child(
                        Rect::from_size((0, 0), (ACTIVITY_LABEL_WIDTH, 1)),
                        TextView::new(format!(" {name}:")),
                    )
                    .child(
                        Rect::from_size((ACTIVITY_LABEL_WIDTH+1, 0), (BAR_WIDTH, 1)),
                        ProgressBar::new()
                            .with_value(counter.clone())
                            .min(0)
                            .max(100)
                            .with_label(label)
                    )
            );
        }

        Panel::new(
            HideableView::new(sub_layout)
                .with_name(Self::view_name(self.index))
        )
        .title(title)
        .title_position(HAlign::Left)
    }

    pub fn set_value(&self, activity: &GpuActivity) {
        self.gfx_counter.set(activity.gfx.unwrap_or(0) as usize);
        self.umc_counter.set(activity.umc.unwrap_or(0) as usize);
        self.media_counter.set(activity.media.unwrap_or(0) as usize);
    }

    fn view_name(index: usize) -> String {
        format!("Activity {index}")
    }

    pub fn cb(siv: &mut cursive::Cursive) {
        use crate::{toggle_view, Opt};

        let indexes = {
            let mut opt = siv.user_data::<Opt>().unwrap().lock().unwrap();
            opt.activity ^= true;

            opt.indexes.clone()
        };

        for i in &indexes {
            let name = Self::view_name(*i);
            siv.call_on_name(&name, toggle_view);
        }
    }
}
