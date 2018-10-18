require 'test_helper'

class HerosControllerTest < ActionDispatch::IntegrationTest

  setup do
    @user = create(:user)
    sign_in @user

    @book = create( :book )
    @adventure = create( :adventure, book: @book, current_page: @book.first_page, user: @user )
  end

  test 'should get show' do
    get heros_url
    assert_response :success
  end

  test 'should update hero' do
    patch heros_url, params: {adventure: { hp: 50 } }
    assert_redirected_to heros_url
  end

end
