require 'test_helper'

class AventuresControllerTest < ActionDispatch::IntegrationTest

  setup do
    @user = create(:user)
    sign_in @user

    @book = create( :book )
    @adventure = create( :adventure, book: @book, current_page: @book.first_page, user: @user, items: {} )
    @user.update!( current_adventure_id: @adventure.id )
  end

  test 'should get index' do
    get list_adventures_url
    assert_response :success
  end

  test 'should get new' do
    get new_adventures_url
    assert_response :success
  end

  test 'should show adventure' do
    get adventures_url
    assert_response :success
  end

  test 'should show play' do
    get play_adventures_url
    assert_response :success
  end

  test 'should start a book' do
    get start_book_adventures_url( @book.book_key )
    assert_redirected_to adventures_url
  end

  test 'should select an adventure' do
    get select_adventures_url( @adventure )
    assert_response :success
  end

  test 'should redirect after play choice' do
    get read_choice_adventures_url( adventure_id: @adventure.id, page_id: @adventure.current_page.page_hash )
    assert_redirected_to play_adventures_url
  end

  test 'should reroll character before adventure' do
    get reroll_adventures_url
    assert_response :success
    get reroll_adventures_url, params: { difficulty: :easy }
    assert_response :success
    get reroll_adventures_url, params: { difficulty: :hard }
    assert_response :success
  end

  test 'should roll dices during adventure' do
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
