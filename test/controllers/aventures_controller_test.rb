require 'test_helper'

class AventuresControllerTest < ActionDispatch::IntegrationTest

  setup do
    @user = create(:user)
    sign_in @user

    @book = create( :book )
    @adventure = create( :adventure, book: @book, current_page: @book.first_page, user: @user )
  end

  test "should get index" do
    get adventures_url
    assert_response :success
  end

  test "should get new" do
    get new_adventure_url
    assert_response :success
  end

  test "should create adventure" do
    assert_difference('Adventure.count') do
      post adventures_url, params: {adventure: {book_id: @adventure.book_id, charisma_avaliable: @adventure.charisma_avaliable, strength: @adventure.strength, gold: @adventure.gold, waterskins: @adventure.waterskins, waterskins_max: @adventure.waterskins_max, hp: @adventure.hp, pages_id: @adventure.page_id, rations: @adventure.rations } }
    end

    assert_redirected_to adventure_url(Adventure.last)
  end

  test "should show adventure" do
    get adventure_url(@adventure)
    assert_response :success
  end

  test "should show play" do
    get adventure_play_url( @adventure )
    assert_response :success
  end

  test "should redirect after play choice" do
    get adventure_read_choice_url( adventure_id: @adventure.id, page_id: @adventure.page_id )
    assert_redirected_to adventure_book_url( @adventure, @adventure.page_id )
  end

  test "should reroll" do
    get reroll_adventures_url( adventure_id: @adventure.id )
    assert_redirected_to adventures_url
  end

  test "should roll dices" do
    get adventure_roll_dices_url( adventure_id: @adventure.id )
    assert_response :success
  end

  test "should destroy adventure" do
    assert_difference('Adventure.count', -1) do
      delete adventure_url(@adventure)
    end

    assert_redirected_to adventures_url
  end
end
