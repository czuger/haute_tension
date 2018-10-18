require 'test_helper'

class NotesControllerTest < ActionDispatch::IntegrationTest

  setup do
    @user = create(:user)
    sign_in @user

    @book = create( :book )
    @adventure = create( :adventure, book: @book, current_page: @book.first_page, user: @user )
  end

  # test "should get edit" do
  #   get edit_note_url( @adventure )
  #   assert_response :success
  # end
  #
  # test "should get update" do
  #   patch note_url( @adventure )
  #   assert_redirected_to adventure_play_url( @adventure )
  # end

end
