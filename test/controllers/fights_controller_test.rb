require 'test_helper'

class FightsControllerTest < ActionDispatch::IntegrationTest

  setup do
    @user = create(:user)
    sign_in @user

    @book = create( :book )
    @fight = create( :fight, book: @book )
    @adventure = create( :adventure, book: @book, current_page: @book.first_page, user: @user, current_fight: @fight )
  end

  test 'should get index' do
    get fights_url
    assert_response :success
  end

  test 'should get fight_monster' do
    # Kernel.stubs( :rand ).returns( 1 )
    get fight_monster_fights_url( monster_index: 1 )
    assert_response :success
  end

end
