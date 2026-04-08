render status: :ok
render json: data, status: :not_found
head :ok
assert_response :not_found
render status: :ok
render json: data, status: :not_found
redirect_to root_path, status: :moved_permanently
render plain: "hello", status: :not_found
render plain: "hello", status: :unauthorized
