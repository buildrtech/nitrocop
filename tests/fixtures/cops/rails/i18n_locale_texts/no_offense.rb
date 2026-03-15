validates :email, presence: { message: :email_missing }
redirect_to root_path, notice: t(".success")
flash[:notice] = t(".success")
mail(to: user.email)
mail(to: user.email, subject: t("mailers.users.welcome"))
validates :name, presence: true

# FP fix: flash as a local variable should not be flagged
# (RuboCop only matches `flash` as a method call, not a local variable)
flash = {}
flash[:error] = "This should not be flagged"
flash[:notice] = "Not flagged when flash is a local var"
