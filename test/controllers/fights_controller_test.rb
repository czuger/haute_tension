require 'test_helper'

class FightsControllerTest < ActionDispatch::IntegrationTest

  setup do
    @user = create(:user)
    sign_in @user

    @book = create( :book )
    @fight = create( :fight, book: @book, user: @user )
    @adventure = create( :adventure, book: @book, current_page: @book.first_page, user: @user, current_fight: @fight, items: {} )
    @user.update!( current_adventure_id: @adventure.id )
  end

  test 'should get index' do
    get fights_url
    assert_response :success
  end

  test 'should list old fights' do
    get old_fights_url
    assert_response :success
  end

  test 'should select an old fights' do
    get select_old_fights_url( @fight )
    assert_redirected_to fights_url
  end

  test 'should get fight_monster' do
    # Kernel.stubs( :rand ).returns( 1 )
    get fight_monster_fights_url( monster_index: 1 )
    assert_response :success
  end

  test 'should get new in case combat already exist' do
    get new_fights_url
    assert_redirected_to fights_url
  end

  test 'should get new in case of new combat' do
    @adventure.update( current_fight_id: nil )
    get new_fights_url
    assert_response :success
  end

  test 'should create new fight' do
    post fights_url, params: { fight: { opponent_1_name: 'Foo', opponent_1_strength: 10, opponent_1_life: 10 } }
    assert_redirected_to fights_url
  end

  test 'should destroy a fight' do
    delete fights_url @fight
    assert_redirected_to new_fights_url
  end

end
