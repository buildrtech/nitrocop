get :index, params: {user_id: 1}, session: {"ACCEPT" => "text/html"}

post :create, params: {name: "foo"}, session: {"X-TOKEN" => "abc"}

put :update, params: {id: 1}, session: {"Authorization" => "Bearer xyz"}

get :new, params: {user_id: 1}
