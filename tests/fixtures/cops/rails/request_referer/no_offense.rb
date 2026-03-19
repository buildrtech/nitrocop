request.referer
request.url
request.host
url = request.referer
redirect_to request.referer

# request with a receiver — RuboCop's NodePattern only matches receiverless `request`
# (send nil? :request), so `Rakismet.request.referrer` should NOT be flagged
Rakismet.request.referrer
SomeModule.request.referrer
obj.request.referrer
