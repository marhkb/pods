use std::borrow::Borrow;

use adw::prelude::ComboRowExt;
use adw::subclass::prelude::*;
use gtk::glib;
use gtk::glib::clone;
use gtk::glib::closure;
use gtk::glib::closure_local;
use gtk::pango;
use gtk::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;

use crate::model;
use crate::utils;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(string = r#"
    <interface>
      <template class="PdsImageLocalComboRow" parent="AdwComboRow">
        <property name="title" translatable="yes">Local Image</property>
        <property name="use-subtitle">True</property>
      </template>
    </interface>
    "#)]
    pub(crate) struct LocalComboRow {
        pub(super) client: glib::WeakRef<model::Client>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for LocalComboRow {
        const NAME: &'static str = "PdsImageLocalComboRow";
        type Type = super::LocalComboRow;
        type ParentType = adw::ComboRow;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for LocalComboRow {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecObject::builder::<model::Client>("client")
                    .flags(glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY)
                    .build()]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "client" => self.obj().set_client(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "client" => self.obj().client().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self) {
            self.parent_constructed();

            let obj = &*self.obj();

            let image_tag_expr = model::Image::this_expression("repo-tags")
                .chain_closure::<String>(closure!(
                    |_: model::Image, repo_tags: gtk::StringList| {
                        utils::escape(&utils::format_option(repo_tags.string(0)))
                    }
                ));
            obj.set_expression(Some(&image_tag_expr));

            let list_factory = gtk::SignalListItemFactory::new();
            list_factory.connect_bind(
                clone!(@weak obj, @to-owned image_tag_expr => move |_, list_item| {
                    let list_item = list_item.downcast_ref::<gtk::ListItem>().unwrap();

                    if let Some(item) = list_item.item() {
                        let label = gtk::Label::builder()
                            .xalign(0.0)
                            .max_width_chars(48)
                            .wrap(true)
                            .wrap_mode(pango::WrapMode::WordChar).build();
                        image_tag_expr.bind(&label, "label", Some(&item));

                        let selected_icon = gtk::Image::builder()
                            .icon_name("object-select-symbolic")
                            .build();

                        adw::ComboRow::this_expression("selected-item")
                            .chain_closure::<bool>(closure_local!(
                                |_: adw::ComboRow, selected: Option<&glib::Object>| {
                                    selected == Some(&item)
                                }
                            ))
                            .bind(&selected_icon, "visible", Some(&obj));

                        let box_ = gtk::Box::builder().spacing(3).build();
                        box_.append(&label);
                        box_.append(&selected_icon);

                        list_item.set_child(Some(&box_));
                    }
                }),
            );
            list_factory.connect_unbind(|_, list_item| {
                list_item
                    .downcast_ref::<gtk::ListItem>()
                    .unwrap()
                    .set_child(gtk::Widget::NONE);
            });
            obj.set_list_factory(Some(&list_factory));
        }
    }

    impl WidgetImpl for LocalComboRow {}
    impl ListBoxRowImpl for LocalComboRow {}
    impl PreferencesRowImpl for LocalComboRow {}
    impl ActionRowImpl for LocalComboRow {}
    impl ComboRowImpl for LocalComboRow {}
}

glib::wrapper! {
    pub(crate) struct LocalComboRow(ObjectSubclass<imp::LocalComboRow>)
        @extends adw::ActionRow, adw::PreferencesRow, gtk::ListBoxRow, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Actionable;
}

impl LocalComboRow {
    pub(crate) fn client(&self) -> Option<model::Client> {
        self.imp().client.upgrade()
    }

    pub(crate) fn set_client(&self, value: Option<&model::Client>) {
        if self.client().as_ref() == value {
            return;
        }

        if let Some(client) = value {
            let model = gtk::SortListModel::new(
                Some(&gtk::FilterListModel::new(
                    Some(client.image_list()),
                    Some(&gtk::CustomFilter::new(|obj| {
                        obj.downcast_ref::<model::Image>()
                            .unwrap()
                            .repo_tags()
                            .n_items()
                            > 0
                    })),
                )),
                Some(&gtk::StringSorter::new(self.expression())),
            );

            self.set_model(Some(&model));
            self.set_sensitive(true);
        }

        self.imp().client.set(value);
        self.notify("client");
    }
}

impl Borrow<adw::ComboRow> for LocalComboRow {
    fn borrow(&self) -> &adw::ComboRow {
        self.upcast_ref()
    }
}

impl AsRef<adw::ComboRow> for LocalComboRow {
    fn as_ref(&self) -> &adw::ComboRow {
        self.upcast_ref()
    }
}

impl From<LocalComboRow> for adw::ComboRow {
    fn from(local_combo_row: LocalComboRow) -> Self {
        local_combo_row.upcast()
    }
}

unsafe impl glib::IsA<adw::ComboRow> for LocalComboRow {}
