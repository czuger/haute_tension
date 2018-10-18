require 'test_helper'

class AventuresControllerTest < ActionDispatch::IntegrationTest

  setup do
    @user = create(:user)
    sign_in @user

    @book = create( :book )
    @adventure = create( :adventure, book: @book, current_page: @book.first_page, user: @user )
  end

  test 'should get index' do
    get list_adventures_url
    assert_response :success
  end

  test 'should get new' do
    get new_adventures_url
    assert_response :success
  end

  test 'should create adventure' do
    assert_difference('Adventure.count') do
      post adventures_url, params: {adventure: {book_id: @adventure.book_id, charisma_avaliable: @adventure.charisma_avaliable, strength: @adventure.strength, gold: @adventure.gold, waterskins: @adventure.waterskins, waterskins_max: @adventure.waterskins_max, hp: @adventure.hp, pages_id: @adventure.current_page_id, rations: @adventure.rations } }
    end

    assert_redirected_to adventures_url
  end

  test 'should show adventure' do
    get adventures_url
    assert_response :success
  end

  test 'should show play' do
    get play_adventures_url
    assert_response :success
  end

  test 'should redirect after play choice' do
    get read_choice_adventures_url( adventure_id: @adventure.id, page_id: @adventure.current_page.page_hash )
    assert_redirected_to play_adventures_url
  end

  test 'should reroll' do
    get reroll_adventures_url
    assert_redirected_to adventures_url
  end

  test 'should roll dices' do
    get roll_dices_adventures_url
    assert_response :success
  end

  test 'should destroy adventure' do
    assert_difference('Adventure.count', -1) do
      delete adventures_url
    end

    assert_redirected_to list_adventures_url
  end
end
