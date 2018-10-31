require 'test_helper'

class BelongingsHistoriesControllerTest < ActionDispatch::IntegrationTest

  setup do
    @user = create(:user)
    sign_in @user

    @book = create( :book )
    @fight = create( :fight, book: @book, user: @user )
    @adventure = create( :adventure, book: @book, current_page: @book.first_page, user: @user, current_fight: @fight, items: {} )
    @user.update!( current_adventure_id: @adventure.id )
    @belongings_history = create( :belongings_history, user: @user, book: @book )
  end
  
  test 'should get show' do
    get belongings_histories_url
    assert_response :success
  end

  test 'should get create' do
    post belongings_histories_url, params: { history_id: @belongings_history.id }
    assert_redirected_to belongings_url
  end

end
