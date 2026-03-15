validates :email, presence: { message: "must be present" }
                                       ^^^^^^^^^^^^^^^^^ Rails/I18nLocaleTexts: Move locale texts to the locale files in the `config/locales` directory.

redirect_to root_path, notice: "Post created!"
                               ^^^^^^^^^^^^^^^ Rails/I18nLocaleTexts: Move locale texts to the locale files in the `config/locales` directory.

mail(to: user.email, subject: "Welcome to My Awesome Site")
                              ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/I18nLocaleTexts: Move locale texts to the locale files in the `config/locales` directory.

flash[:notice] = "Post created!"
                 ^^^^^^^^^^^^^^^ Rails/I18nLocaleTexts: Move locale texts to the locale files in the `config/locales` directory.

flash.now[:alert] = "Something went wrong!"
                    ^^^^^^^^^^^^^^^^^^^^^^^ Rails/I18nLocaleTexts: Move locale texts to the locale files in the `config/locales` directory.

# FN fix: validates with message at top level (not nested in hash)
Topic.validates :title, confirmation: true, message: "Y U NO CONFIRM"
                                                     ^^^^^^^^^^^^^^^^ Rails/I18nLocaleTexts: Move locale texts to the locale files in the `config/locales` directory.

# FN fix: redirect_to with flash: { notice: "string" }
redirect_to root_path, flash: { notice: "User not found" }
                                        ^^^^^^^^^^^^^^^^ Rails/I18nLocaleTexts: Move locale texts to the locale files in the `config/locales` directory.

# FN fix: redirect_to with notice passed to URL helper (recursive search)
redirect_to admin_orders_url(notice: "Order was successfully created.")
                                     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/I18nLocaleTexts: Move locale texts to the locale files in the `config/locales` directory.

# FN fix: mail with subject in a hash literal argument
mail({ subject: "The first email on new API!" }.merge!(hash))
                ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/I18nLocaleTexts: Move locale texts to the locale files in the `config/locales` directory.

# FN fix: redirect_to with explicit hash containing flash alert
redirect_to [:admin, @edition], { flash: { alert: "This is historic content" } }
                                                  ^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/I18nLocaleTexts: Move locale texts to the locale files in the `config/locales` directory.
