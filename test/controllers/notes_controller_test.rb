require 'test_helper'

class NotesControllerTest < ActionDispatch::IntegrationTest

  setup do
    @user = create(:user)
    sign_in @user

    @book = create( :book )
    @adventure = create( :adventure, book: @book, current_page: @book.first_page, user: @user, items: {} )
    @user.update!( current_adventure_id: @adventure.id )
  end

  test 'should get edit' do
    get edit_notes_url
    assert_response :success
  end

  test 'should get update' do
    patch notes_url
    assert_redirected_to play_adventures_url
  end

end
