require 'test_helper'

class BelongingsHistoriesControllerTest < ActionDispatch::IntegrationTest
  test "should get show" do
    get belongings_histories_show_url
    assert_response :success
  end

  test "should get create" do
    get belongings_histories_create_url
    assert_response :success
  end

end
