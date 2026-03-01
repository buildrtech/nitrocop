I18n.t("foo.bar.baz")
I18n.t(:foo)
I18n.t("foo")
I18n.translate("users.show.title")
t(:hello)
t("simple_key")
t("admin.reports.processed_msg", id: 1)
I18n.t(:subject, scope: ['devise.mailer', action_name])
t(activity.browser, scope: 'sessions.browsers', default: activity.browser.to_s)
I18n.t('activerecord.errors.messages.record_invalid')
# Non-literal first argument — RuboCop requires first arg to be symbol or string
t key1, scope: :one
I18n.t(format, scope: :user)
I18n.t(feature_key, scope: :"ee.features")
I18n.t(status_name.to_sym, scope: :user)
t(variable, scope: [:foo, :bar])
t [:key1, :key2], scope: :one
# Namespaced receiver — Formtastic::I18n is not plain I18n
Formtastic::I18n.t(:duck, scope: [:custom_scope])
